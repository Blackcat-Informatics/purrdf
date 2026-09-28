// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A variable-endpoint `SERVICE ?e { … }` answered over the endpoints the pattern before
//! it binds.
//!
//! # What the specification says
//!
//! SPARQL 1.1 Federated Query §4, "SERVICE Variables", is explicitly informative: "we do
//! not present official evaluation semantics for the SPARQL pattern `SERVICE VAR`". What
//! it does say is that "a variable used in place of a service IRI indicates that the
//! service call for any solution depends on that variable's binding in that solution",
//! that such a clause "can be executed as a series of separate invocations of SPARQL
//! query services" whose results "are combined using union", and that "the query engine
//! must determine the possible target SPARQL query services … Execution order may also be
//! used to determine the list of services to be tried".
//!
//! # What this engine does with it
//!
//! Two evaluation routes, one meaning:
//!
//! * **A pattern earlier in the same group** — `?s ex:endpoint ?e . SERVICE ?e { … }`,
//!   including `VALUES`, `BIND` and an explicit `LATERAL` — is parsed into a `LATERAL`
//!   join, and [`crate::binop::eval_lateral`] substitutes each left solution's IRI into
//!   the clause (the per-solution dependency the section describes).
//! * **The left side of a group join, an `OPTIONAL` or a `MINUS`** — `{ ?s ex:endpoint
//!   ?e } { SERVICE ?e { … } }`, `?s ex:endpoint ?e OPTIONAL { SERVICE ?e { … } }`,
//!   `?s ex:endpoint ?e MINUS { SERVICE ?e { … } }` — is the case this module serves. The
//!   algebra evaluates those right operands independently of the left, so no solution is
//!   substituted in. Instead the list of services to be tried is taken from the
//!   execution order, as the section permits: the distinct IRIs `?e` takes in the left
//!   operand's solutions, in first-occurrence order. The clause becomes the union, over
//!   that list, of `{?e ↦ iri} ⋈ Invocation(iri, P)` — one request per distinct
//!   endpoint — and the enclosing operator then runs its own, unmodified algebra over it.
//! * **The other side of a group join** — `{ SERVICE ?e { … } ?s ex:endpoint ?e }`,
//!   `{ SERVICE ?e { … } VALUES ?e { … } }`. A join is commutative, so the side that
//!   binds `?e` is evaluated first and supplies the list, and the join then runs as
//!   written ([`binds_left_endpoints`]). `OPTIONAL` and `MINUS` do not commute: their
//!   list comes from the left alone.
//!
//! Restricting the union to that list is not an approximation, and that is what licenses
//! it. Every left solution binds `?e` (a left operand that leaves `?e` unbound in some
//! solution is refused, see below), and every row the clause produces binds `?e` to the
//! endpoint that produced it. So a row from an endpoint outside the list is incompatible
//! with every left solution: it could join none of them (group join, `OPTIONAL`), and it
//! could remove none of them (`MINUS`, whose removal requires compatibility). The answer
//! is therefore exactly the one the union over every conceivable endpoint would give.
//! That argument holds only while every row the clause produces carries `?e` up to the
//! enclosing operator, which is why the positions it is used in are the ones
//! [`served_endpoint_variables`] classifies as direct.
//!
//! # `SILENT`, and `MINUS`
//!
//! A `SILENT` invocation that does not succeed — the endpoint fails, is refused, has no
//! source, or the value `?e` takes is not an IRI — is Ω0 for that invocation alone
//! (Federated Query §3.2), recorded on the evidence. Under a group join or an `OPTIONAL`
//! the union formulation carries it as `{?e ↦ iri}`: it joins the left solutions bound to
//! that endpoint exactly as Ω0 would, and no other. `OPTIONAL` keeps a left solution whose
//! endpoint answers nothing, as it always does.
//!
//! `MINUS` cannot use the union formulation for a silenced endpoint: `{?e ↦ iri}` shares
//! `?e` with every left solution bound to that endpoint and would remove them all, where
//! Ω0 — which binds nothing — removes nothing. So under `MINUS` the left solutions are
//! partitioned by their endpoint, the right operand is evaluated once per partition with
//! that endpoint alone listed ([`minus_partitions`]), each partition is subtracted by its
//! own result, and the survivors are merged back in left-solution order. A silenced
//! endpoint contributes Ω0 to its own partition only. It is still one request per
//! endpoint.
//!
//! # What is refused
//!
//! A `SERVICE ?e` evaluated where no solution names an endpoint has no endpoint to be sent
//! to, and makes no invocation. Returning the join identity there would make the
//! enclosing join a no-op and the answer look complete when nothing was asked, so the
//! refusal is a hard [`EvalError::Unsupported`] that names the shapes that do evaluate
//! and the rewrite, with or without `SILENT`: `SILENT` absorbs an invocation that fails,
//! and there is none.

use std::convert::Infallible;
use std::fmt::Write as _;
use std::ops::ControlFlow;
use std::sync::Arc;

use purrdf_core::{DatasetView, TermValue, TermVisit};
use purrdf_sparql_algebra::{Expression, GraphPattern, NamedNodePattern, Variable};

use crate::error::EvalError;
use crate::eval::{EvalCtx, eval_evaluated};
use crate::governor::lift::{Evaluated, Truncation};
use crate::governor::soundness::{ExpressionPart, PatternPart};
use crate::plan::NodeId;
use crate::remote::Invocation;
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq, VarSchema};

/// Whether the query being evaluated contains a variable-endpoint `SERVICE`, and if it
/// does, where it is served.
///
/// Computed once per evaluated tree, with the tree ([`crate::plan::Tree::build`]), so a
/// query without one — nearly every query — pays one allocation-free walk of its algebra
/// and nothing per join, and a query with one pays one more walk to build the
/// [`ServedIndex`] and an index read per join.
#[derive(Debug, Clone)]
pub(crate) enum EndpointScan {
    /// The query has no variable-endpoint `SERVICE`: nothing to analyse.
    Absent,
    /// The query has one; the index answers for every operand of the query as written.
    Present(Arc<ServedIndex>),
}

impl EndpointScan {
    /// Whether the query has no variable-endpoint `SERVICE`.
    pub(crate) const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
}

/// The endpoint analysis of one tree's algebra, indexed by [`NodeId`].
///
/// Only nodes of the tree the scan walked have entries. A pattern built during evaluation
/// — a `LATERAL`'s substituted right operand, a deferred `EXISTS` body's copy, a
/// user-defined function's body — is not a node of it, and is analysed where it is
/// evaluated, as is every operand evaluated while a deferred `EXISTS` placeholder is in
/// scope (the placeholder's substitution decides what its body serves, and the scan
/// cannot see it).
#[derive(Debug, Default)]
pub(crate) struct ServedIndex {
    /// Every operand of a group join (both sides), an `OPTIONAL` (right) and a `MINUS`
    /// (right): the endpoint variables served in it (see [`served_endpoint_variables`]).
    served: Vec<Option<Arc<[Variable]>>>,
    /// Every `EXISTS` body: the variable-endpoint `SERVICE` variables in it that no
    /// `SELECT` inside it hides.
    exists_uses: Vec<Option<Arc<[Variable]>>>,
}

impl ServedIndex {
    /// An index with no entries over a tree of `len` nodes.
    fn with_len(len: usize) -> Self {
        Self {
            served: vec![None; len],
            exists_uses: vec![None; len],
        }
    }

    /// The variable-endpoint `SERVICE` variables in `body`, an `EXISTS` body of the
    /// scanned tree, or `None` when `body` is not one.
    pub(crate) fn exists_uses(&self, body: NodeId) -> Option<&[Variable]> {
        self.exists_uses
            .get(body.index())
            .and_then(Option::as_deref)
    }

    /// The endpoint variables served in `operand`, an operand of the scanned tree, or
    /// `None` when `operand` is not one.
    fn served_at(&self, operand: NodeId) -> Option<&Arc<[Variable]>> {
        self.served.get(operand.index()).and_then(Option::as_ref)
    }
}

/// The id a scan records `node` under: every node the scan reaches is a node of the tree
/// whose address map it was handed.
fn scanned_id(ids: &crate::DetHashMap<usize, NodeId>, node: &GraphPattern) -> NodeId {
    *ids.get(&(std::ptr::from_ref(node) as usize))
        .expect("the endpoint scan walks only nodes of the tree it indexes")
}

/// What a variable-endpoint `SERVICE` finds for its variable in the enclosing frames.
#[derive(Debug, Clone)]
pub(crate) enum EndpointBinding<I> {
    /// The distinct terms the left operand of the enclosing join binds the variable to,
    /// in first-occurrence order.
    Endpoints(Arc<[SolutionTerm<I>]>),
    /// The left operand binds the variable in some of its solutions but not all.
    PartlyUnbound,
    /// A sub-`SELECT` between the frame and the `SERVICE` does not project the variable:
    /// the `SERVICE` names a different variable of the same name.
    Hidden,
}

/// Which operator an endpoint frame belongs to, which decides what a silenced
/// invocation contributes to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameRole {
    /// The right operand of a group join, or its left operand when the join takes its
    /// endpoints from the right: a silenced endpoint contributes `{?v ↦ iri}`.
    Join,
    /// The right operand of an `OPTIONAL`: as [`Self::Join`].
    Optional,
    /// One partition of a `MINUS`'s left solutions: a silenced endpoint contributes Ω0,
    /// which removes nothing.
    Minus,
}

/// One enclosing operator's binding for one endpoint variable.
#[derive(Debug, Clone)]
pub(crate) struct EndpointFrame<I> {
    /// The endpoint variable.
    variable: Variable,
    /// What the operator binds it to.
    binding: EndpointBinding<I>,
    /// The operator.
    role: FrameRole,
}

/// How a `SERVICE ?v` inside an operand is reached from the operand's root.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
struct EndpointUse {
    /// The endpoint variable.
    variable: Variable,
    /// Reached through operators whose every row carries `?v` upwards.
    direct: bool,
    /// Also reached under an operator that could absorb, drop or re-select rows carrying
    /// `?v` (a nested `OPTIONAL` or `MINUS` right side, an `EXISTS`, a `LIMIT`/`OFFSET`,
    /// an aggregate that does not group by `?v`).
    conflict: bool,
}

/// A projection the walk has descended through.
#[derive(Clone, Copy)]
enum Scope<'a> {
    /// A `SELECT` list: a variable it does not name is a different variable below it.
    Project(&'a [Variable]),
    /// A `GROUP BY` list: rows below it reach the enclosing operator only through an
    /// aggregate unless the variable is a grouping key.
    Group(&'a [Variable]),
}

/// The endpoint variables of `right` this module can serve from the enclosing operator's
/// left operand: those of a `SERVICE ?v` reached only in direct positions (see the module
/// doc for why only those).
fn served_endpoint_variables(
    right: &GraphPattern,
    placeholders: Option<&crate::deferred_exists::DeferredMap>,
) -> Vec<Variable> {
    let mut uses = Vec::new();
    classify(right, true, &mut uses, placeholders);
    uses.into_iter()
        .filter(|u| u.direct && !u.conflict)
        .map(|u| u.variable)
        .collect()
}

/// Record one reached `SERVICE ?variable`.
fn record(variable: &Variable, direct: bool, scopes: &[Scope<'_>], uses: &mut Vec<EndpointUse>) {
    let mut direct = direct;
    for scope in scopes {
        match scope {
            Scope::Project(vars) if !vars.contains(variable) => return,
            Scope::Group(vars) if !vars.contains(variable) => direct = false,
            Scope::Project(_) | Scope::Group(_) => {}
        }
    }
    let index = if let Some(i) = uses.iter().position(|u| &u.variable == variable) {
        i
    } else {
        uses.push(EndpointUse {
            variable: variable.clone(),
            direct: false,
            conflict: false,
        });
        uses.len() - 1
    };
    if direct {
        uses[index].direct = true;
    } else {
        uses[index].conflict = true;
    }
}

/// One step of [`classify`]'s walk.
enum ClassifyStep<'a> {
    /// A pattern, and whether it stands in a direct position.
    Pattern(&'a GraphPattern, bool),
    /// An expression whose `EXISTS` patterns are classified.
    Expression(&'a Expression),
    /// An `EXISTS` body reached through an expression: a deferred placeholder answers
    /// from its site, any other body is classified as written, never direct.
    Exists(&'a GraphPattern),
    /// The `SELECT` or `GROUP BY` list entered last is left: the pattern under it is
    /// classified.
    LeaveScope,
}

/// Classify every `SERVICE ?v` reachable from `pattern`. `direct` says whether `pattern`
/// itself is in a direct position.
///
/// A loop over an explicit work list, so an operand of any depth costs heap and never
/// stack. The list pops each node's parts in the order the node evaluates them — an
/// operator's inner pattern before its expressions, both operands left to right, a
/// `SELECT` or `GROUP BY` list entered before its inner pattern and left after it — so
/// `uses` fills in that order.
///
/// The match is wildcard-free so a new algebra variant is a compile error here rather
/// than a position silently classified.
fn classify(
    pattern: &GraphPattern,
    direct: bool,
    uses: &mut Vec<EndpointUse>,
    placeholders: Option<&crate::deferred_exists::DeferredMap>,
) {
    let mut scopes: Vec<Scope<'_>> = Vec::new();
    let mut pending = vec![ClassifyStep::Pattern(pattern, direct)];
    while let Some(step) = pending.pop() {
        match step {
            ClassifyStep::Pattern(pattern, direct) => {
                match pattern {
                    GraphPattern::Bgp { .. }
                    | GraphPattern::Path { .. }
                    | GraphPattern::Values { .. }
                    | GraphPattern::PropertyFunction(_) => {}
                    // The body is forwarded as text, never evaluated here, so a `SERVICE`
                    // nested in it is the remote endpoint's to resolve.
                    GraphPattern::Service { name, .. } => {
                        if let NamedNodePattern::Variable(variable) = name {
                            record(variable, direct, &scopes, uses);
                        }
                    }
                    GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
                        pending.push(ClassifyStep::Pattern(right, direct));
                        pending.push(ClassifyStep::Pattern(left, direct));
                    }
                    GraphPattern::Union { arms } => {
                        pending.extend(
                            arms.iter()
                                .rev()
                                .map(|arm| ClassifyStep::Pattern(arm, direct)),
                        );
                    }
                    GraphPattern::LeftJoin {
                        left,
                        right,
                        expression,
                    } => {
                        if let Some(expression) = expression {
                            pending.push(ClassifyStep::Expression(expression));
                        }
                        pending.push(ClassifyStep::Pattern(right, false));
                        pending.push(ClassifyStep::Pattern(left, direct));
                    }
                    GraphPattern::Minus { left, right } => {
                        pending.push(ClassifyStep::Pattern(right, false));
                        pending.push(ClassifyStep::Pattern(left, direct));
                    }
                    GraphPattern::Filter { expr, inner } => {
                        pending.push(ClassifyStep::Expression(expr));
                        pending.push(ClassifyStep::Pattern(inner, direct));
                    }
                    GraphPattern::Extend {
                        inner, expression, ..
                    }
                    | GraphPattern::Unfold {
                        inner, expression, ..
                    } => {
                        pending.push(ClassifyStep::Expression(expression));
                        pending.push(ClassifyStep::Pattern(inner, direct));
                    }
                    GraphPattern::Graph { inner, .. }
                    | GraphPattern::Distinct { inner }
                    | GraphPattern::Reduced { inner } => {
                        pending.push(ClassifyStep::Pattern(inner, direct));
                    }
                    GraphPattern::OrderBy { inner, expression } => {
                        pending.extend(expression.iter().rev().map(|key| {
                            ClassifyStep::Expression(crate::modifier::order_sort_key(key))
                        }));
                        pending.push(ClassifyStep::Pattern(inner, direct));
                    }
                    GraphPattern::Project { inner, variables } => {
                        scopes.push(Scope::Project(variables));
                        pending.push(ClassifyStep::LeaveScope);
                        pending.push(ClassifyStep::Pattern(inner, direct));
                    }
                    // A slice keeps a positional selection of its input, and which rows it
                    // keeps depends on every row before them — including rows from endpoints
                    // outside the list. The identity slice selects nothing.
                    GraphPattern::Slice {
                        inner,
                        start,
                        length,
                    } => {
                        let identity = *start == 0 && length.is_none();
                        pending.push(ClassifyStep::Pattern(inner, direct && identity));
                    }
                    // The aggregates' expressions are classified once the list is left.
                    GraphPattern::Group {
                        inner,
                        variables,
                        aggregates,
                    } => {
                        scopes.push(Scope::Group(variables));
                        let first = pending.len();
                        for (_, aggregate) in aggregates {
                            for e in aggregate.args().iter().chain(
                                aggregate
                                    .order_by()
                                    .iter()
                                    .map(crate::modifier::order_sort_key),
                            ) {
                                pending.push(ClassifyStep::Expression(e));
                            }
                        }
                        pending[first..].reverse();
                        pending.push(ClassifyStep::LeaveScope);
                        pending.push(ClassifyStep::Pattern(inner, direct));
                    }
                }
            }
            // The `SERVICE ?v` clauses inside an expression's `EXISTS` patterns are never
            // direct, since an `EXISTS` answers a boolean rather than carrying rows
            // upwards.
            ClassifyStep::Expression(expr) => {
                let first = pending.len();
                crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
                    match part {
                        ExpressionPart::Sub(sub) => pending.push(ClassifyStep::Expression(sub)),
                        ExpressionPart::Exists(body) => pending.push(ClassifyStep::Exists(body)),
                        ExpressionPart::Call(_) => {}
                    }
                    false
                });
                pending[first..].reverse();
            }
            // A substituted copy's placeholder stands for a body the walk cannot see: its
            // site lists the body's `SERVICE ?v` clauses, and the substitution it is owed
            // decides which of them are still variable endpoints.
            ClassifyStep::Exists(body) => {
                match placeholders.and_then(|map| map.get(&(std::ptr::from_ref(body) as usize))) {
                    Some(slot) => {
                        for variable in &slot.site.service_uses {
                            if !slot.env.resolves_endpoint(variable) {
                                record(variable, false, &scopes, uses);
                            }
                        }
                    }
                    None => pending.push(ClassifyStep::Pattern(body, false)),
                }
            }
            ClassifyStep::LeaveScope => {
                scopes.pop();
            }
        }
    }
}

/// One node of [`mentions_variable_endpoint`]'s work list.
enum ScanNode<'a> {
    /// A pattern.
    Pattern(&'a GraphPattern),
    /// An expression whose `EXISTS` patterns are scanned.
    Expression(&'a Expression),
}

/// Whether `pattern` contains a variable-endpoint `SERVICE` anywhere a local evaluation
/// can reach — every position, projections and `EXISTS` included, so the answer is
/// never "absent" for a clause some inner join could serve.
///
/// A loop over a work list that keeps a shallow query's pending nodes inline, so the
/// scan every evaluation runs allocates nothing for one and never needs more stack for
/// a deeper query.
pub(crate) fn mentions_variable_endpoint(pattern: &GraphPattern) -> bool {
    let mut pending: smallvec::SmallVec<[ScanNode<'_>; 16]> = smallvec::SmallVec::new();
    pending.push(ScanNode::Pattern(pattern));
    while let Some(node) = pending.pop() {
        match node {
            // A `SERVICE` body is forwarded as text, never evaluated here, so it is not
            // entered.
            ScanNode::Pattern(GraphPattern::Service { name, .. }) => {
                if matches!(name, NamedNodePattern::Variable(_)) {
                    return true;
                }
            }
            ScanNode::Pattern(pattern) => {
                crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
                    pending.push(match part {
                        PatternPart::Child(child, _) => ScanNode::Pattern(child),
                        PatternPart::Expression(expr) => ScanNode::Expression(expr),
                    });
                    false
                });
            }
            ScanNode::Expression(expr) => {
                crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
                    match part {
                        ExpressionPart::Sub(sub) => pending.push(ScanNode::Expression(sub)),
                        ExpressionPart::Exists(body) => pending.push(ScanNode::Pattern(body)),
                        ExpressionPart::Call(_) => {}
                    }
                    false
                });
            }
        }
    }
    false
}

/// The scan of the tree rooted at `pattern`, of `len` nodes, whose node addresses `ids`
/// maps to their ids — built with the tree by [`crate::plan::Tree::build`].
pub(crate) fn scan(
    pattern: &GraphPattern,
    ids: &crate::DetHashMap<usize, NodeId>,
    len: usize,
) -> EndpointScan {
    if !mentions_variable_endpoint(pattern) {
        return EndpointScan::Absent;
    }
    let mut index = ServedIndex::with_len(len);
    index.summarize(pattern, ids);
    EndpointScan::Present(Arc::new(index))
}

/// One variable's variable-endpoint `SERVICE` occurrences below a node, relative to that
/// node: `direct` when some occurrence carries its rows up to it, `conflict` when some
/// occurrence reaches it only through an operator that could absorb, drop or re-select
/// them. The bottom-up form of [`classify`].
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
struct Occurrence {
    variable: Variable,
    direct: bool,
    conflict: bool,
}

/// Fold `from` into `into`, one entry per variable.
fn merge(into: &mut Vec<Occurrence>, from: Vec<Occurrence>) {
    for occurrence in from {
        if let Some(existing) = into
            .iter_mut()
            .find(|existing| existing.variable == occurrence.variable)
        {
            existing.direct |= occurrence.direct;
            existing.conflict |= occurrence.conflict;
        } else {
            into.push(occurrence);
        }
    }
}

/// `summary` seen through an operator that does not carry its rows upwards.
fn indirect(mut summary: Vec<Occurrence>) -> Vec<Occurrence> {
    for occurrence in &mut summary {
        occurrence.conflict |= occurrence.direct;
        occurrence.direct = false;
    }
    summary
}

impl ServedIndex {
    /// Record `operand`'s served variables.
    fn note(
        &mut self,
        operand: &GraphPattern,
        summary: &[Occurrence],
        ids: &crate::DetHashMap<usize, NodeId>,
    ) {
        let served: Arc<[Variable]> = summary
            .iter()
            .filter(|o| o.direct && !o.conflict)
            .map(|o| o.variable.clone())
            .collect();
        self.served[scanned_id(ids, operand).index()] = Some(served);
    }

    /// The occurrences below `pattern`, indexing every operand and `EXISTS` body on the
    /// way. Wildcard-free, like [`classify`], whose classification it computes.
    ///
    /// A loop over an explicit frame stack and a stack of the summaries computed so
    /// far: a node is entered, its parts are pushed above it in the order the node
    /// evaluates them ([`summary_parts`]), and once every part has left its summary on
    /// the summary stack the node is exited and combines them. The order the parts'
    /// summaries are merged in is the order the operator evaluates the parts, so the
    /// served lists and `EXISTS` uses come out in that order.
    fn summarize(
        &mut self,
        pattern: &GraphPattern,
        ids: &crate::DetHashMap<usize, NodeId>,
    ) -> Vec<Occurrence> {
        let mut frames = vec![SummaryFrame::Enter(SummaryNode::Pattern(pattern))];
        let mut summaries: Vec<Vec<Occurrence>> = Vec::new();
        while let Some(frame) = frames.pop() {
            match frame {
                SummaryFrame::Enter(node) => {
                    let exit = frames.len();
                    frames.push(SummaryFrame::Exit(node, 0));
                    let first = frames.len();
                    summary_parts(node, &mut |part| frames.push(SummaryFrame::Enter(part)));
                    let parts = frames.len() - first;
                    frames[first..].reverse();
                    frames[exit] = SummaryFrame::Exit(node, parts);
                }
                SummaryFrame::Exit(node, parts) => {
                    let start = summaries.len() - parts;
                    let mut kids = summaries.split_off(start).into_iter();
                    let summary = self.combine(node, &mut kids, ids);
                    summaries.push(summary);
                }
            }
        }
        summaries
            .pop()
            .expect("the root's summary is the last one computed")
    }

    /// The summary of `node` from its parts' summaries, `kids`, in the order
    /// [`summary_parts`] pushed the parts.
    fn combine(
        &mut self,
        node: SummaryNode<'_>,
        kids: &mut std::vec::IntoIter<Vec<Occurrence>>,
        ids: &crate::DetHashMap<usize, NodeId>,
    ) -> Vec<Occurrence> {
        fn part(kids: &mut std::vec::IntoIter<Vec<Occurrence>>) -> Vec<Occurrence> {
            kids.next().expect("every part pushed its summary")
        }
        match node {
            SummaryNode::Pattern(pattern) => match pattern {
                GraphPattern::Bgp { .. }
                | GraphPattern::Path { .. }
                | GraphPattern::Values { .. }
                | GraphPattern::PropertyFunction(_) => Vec::new(),
                GraphPattern::Service { name, .. } => match name {
                    NamedNodePattern::Variable(variable) => vec![Occurrence {
                        variable: variable.clone(),
                        direct: true,
                        conflict: false,
                    }],
                    NamedNodePattern::NamedNode(_) => Vec::new(),
                },
                GraphPattern::Join { left, right } => {
                    let mut summary = part(kids);
                    let right_summary = part(kids);
                    self.note(left, &summary, ids);
                    self.note(right, &right_summary, ids);
                    merge(&mut summary, right_summary);
                    summary
                }
                GraphPattern::Lateral { .. } => {
                    let mut summary = part(kids);
                    let right_summary = part(kids);
                    merge(&mut summary, right_summary);
                    summary
                }
                GraphPattern::Union { .. } => {
                    let mut summary = Vec::new();
                    for arm_summary in kids.by_ref() {
                        merge(&mut summary, arm_summary);
                    }
                    summary
                }
                GraphPattern::LeftJoin {
                    right, expression, ..
                } => {
                    let mut summary = part(kids);
                    let right_summary = part(kids);
                    self.note(right, &right_summary, ids);
                    merge(&mut summary, indirect(right_summary));
                    if expression.is_some() {
                        let expression_summary = part(kids);
                        merge(&mut summary, expression_summary);
                    }
                    summary
                }
                GraphPattern::Minus { right, .. } => {
                    let mut summary = part(kids);
                    let right_summary = part(kids);
                    self.note(right, &right_summary, ids);
                    merge(&mut summary, indirect(right_summary));
                    summary
                }
                GraphPattern::Filter { .. }
                | GraphPattern::Extend { .. }
                | GraphPattern::Unfold { .. } => {
                    let mut summary = part(kids);
                    let expression_summary = part(kids);
                    merge(&mut summary, expression_summary);
                    summary
                }
                GraphPattern::Graph { .. }
                | GraphPattern::Distinct { .. }
                | GraphPattern::Reduced { .. } => part(kids),
                GraphPattern::OrderBy { .. } => {
                    let mut summary = part(kids);
                    for key_summary in kids.by_ref() {
                        merge(&mut summary, key_summary);
                    }
                    summary
                }
                GraphPattern::Project { variables, .. } => {
                    let mut summary = part(kids);
                    summary.retain(|o| variables.contains(&o.variable));
                    summary
                }
                GraphPattern::Slice { start, length, .. } => {
                    let summary = part(kids);
                    if *start == 0 && length.is_none() {
                        summary
                    } else {
                        indirect(summary)
                    }
                }
                GraphPattern::Group { variables, .. } => {
                    let mut summary = part(kids);
                    for occurrence in &mut summary {
                        if !variables.contains(&occurrence.variable) {
                            occurrence.conflict |= occurrence.direct;
                            occurrence.direct = false;
                        }
                    }
                    for expression_summary in kids.by_ref() {
                        merge(&mut summary, expression_summary);
                    }
                    summary
                }
            },
            // The occurrences inside an expression's `EXISTS` patterns: never direct.
            SummaryNode::Expression(_) => {
                let mut summary = Vec::new();
                for part_summary in kids.by_ref() {
                    merge(&mut summary, part_summary);
                }
                summary
            }
            SummaryNode::Exists(body) => {
                let body_summary = part(kids);
                let uses: Arc<[Variable]> =
                    body_summary.iter().map(|o| o.variable.clone()).collect();
                self.exists_uses[scanned_id(ids, body).index()] = Some(uses);
                indirect(body_summary)
            }
        }
    }
}

/// One node of [`ServedIndex::summarize`]'s walk.
#[derive(Clone, Copy)]
enum SummaryNode<'a> {
    /// A pattern, summarized by its variant.
    Pattern(&'a GraphPattern),
    /// An expression, summarized as the merge of its parts' summaries.
    Expression(&'a Expression),
    /// An `EXISTS` body reached through an expression: its summary is recorded as the
    /// body's uses and merged as never direct.
    Exists(&'a GraphPattern),
}

/// One frame of that walk: a node entered, whose parts are pushed above it, or a node
/// exited, whose parts' summaries — that many of them — top the summary stack.
enum SummaryFrame<'a> {
    Enter(SummaryNode<'a>),
    Exit(SummaryNode<'a>, usize),
}

/// Push the parts of `node` whose summaries [`ServedIndex::combine`] reads, in the order
/// it reads them: the order the node evaluates them.
///
/// Wildcard-free, so a new algebra variant is a compile error here rather than a node
/// whose parts are silently skipped.
fn summary_parts<'a>(node: SummaryNode<'a>, push: &mut impl FnMut(SummaryNode<'a>)) {
    match node {
        SummaryNode::Pattern(pattern) => match pattern {
            GraphPattern::Bgp { .. }
            | GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::PropertyFunction(_)
            | GraphPattern::Service { .. } => {}
            GraphPattern::Join { left, right }
            | GraphPattern::Lateral { left, right }
            | GraphPattern::Minus { left, right } => {
                push(SummaryNode::Pattern(left));
                push(SummaryNode::Pattern(right));
            }
            GraphPattern::Union { arms } => {
                for arm in arms {
                    push(SummaryNode::Pattern(arm));
                }
            }
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                push(SummaryNode::Pattern(left));
                push(SummaryNode::Pattern(right));
                if let Some(expression) = expression {
                    push(SummaryNode::Expression(expression));
                }
            }
            GraphPattern::Filter { expr, inner } => {
                push(SummaryNode::Pattern(inner));
                push(SummaryNode::Expression(expr));
            }
            GraphPattern::Extend {
                inner, expression, ..
            }
            | GraphPattern::Unfold {
                inner, expression, ..
            } => {
                push(SummaryNode::Pattern(inner));
                push(SummaryNode::Expression(expression));
            }
            GraphPattern::Graph { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::Project { inner, .. }
            | GraphPattern::Slice { inner, .. } => push(SummaryNode::Pattern(inner)),
            GraphPattern::OrderBy { inner, expression } => {
                push(SummaryNode::Pattern(inner));
                for key in expression {
                    push(SummaryNode::Expression(crate::modifier::order_sort_key(
                        key,
                    )));
                }
            }
            GraphPattern::Group {
                inner, aggregates, ..
            } => {
                push(SummaryNode::Pattern(inner));
                for (_, aggregate) in aggregates {
                    for e in aggregate.args().iter().chain(
                        aggregate
                            .order_by()
                            .iter()
                            .map(crate::modifier::order_sort_key),
                    ) {
                        push(SummaryNode::Expression(e));
                    }
                }
            }
        },
        SummaryNode::Expression(expr) => {
            crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
                match part {
                    ExpressionPart::Sub(sub) => push(SummaryNode::Expression(sub)),
                    ExpressionPart::Exists(body) => push(SummaryNode::Exists(body)),
                    ExpressionPart::Call(_) => {}
                }
                false
            });
        }
        SummaryNode::Exists(body) => push(SummaryNode::Pattern(body)),
    }
}

/// The endpoint variables served in `operand` (see [`served_endpoint_variables`]): read
/// from the query's index when `operand` is one of its nodes and no deferred `EXISTS`
/// placeholder is in scope, and analysed here otherwise.
fn served_in<D: DatasetView + Sync>(
    operand: &GraphPattern,
    index: &ServedIndex,
    ctx: &EvalCtx<'_, D>,
) -> Arc<[Variable]> {
    if ctx.deferred_exists.is_none()
        && let Some(served) = ctx
            .plan_node(operand)
            .and_then(|operand| index.served_at(operand))
    {
        return Arc::clone(served);
    }
    served_endpoint_variables(operand, ctx.deferred_exists.as_deref()).into()
}

/// The innermost frame for `variable`, if any frame names it.
fn frame_for<'c, I>(
    frames: &'c [EndpointFrame<I>],
    variable: &Variable,
) -> Option<&'c EndpointFrame<I>> {
    frames
        .iter()
        .rev()
        .find(|frame| &frame.variable == variable)
}

/// The innermost frame's binding for `variable`, if any frame names it.
fn binding_for<'c, I>(
    frames: &'c [EndpointFrame<I>],
    variable: &Variable,
) -> Option<&'c EndpointBinding<I>> {
    frame_for(frames, variable).map(|frame| &frame.binding)
}

/// Evaluate `right`, the right operand of a group join, an `OPTIONAL` or a `MINUS` whose
/// left operand produced `left`, with every variable-endpoint `SERVICE` in a direct
/// position of `right` answered over the endpoints `left` binds. `role` names the
/// operator.
///
/// # Errors
///
/// Whatever evaluating `right` raises.
pub(crate) fn eval_right_operand<D: DatasetView + Sync>(
    left: &SolutionSeq<D::Id>,
    right: &GraphPattern,
    role: FrameRole,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let EndpointScan::Present(index) = ctx.endpoint_scan() else {
        return eval_evaluated(right, ctx);
    };
    let index = Arc::clone(index);
    let served = served_in(right, &index, ctx);
    if served.is_empty() {
        return eval_evaluated(right, ctx);
    }
    let depth = ctx.endpoint_frames.len();
    for variable in served.iter() {
        let column = left.schema.index_of(variable);
        let binding = match column {
            Some(c) if left.rows.iter().all(|row| row[c].is_some()) => {
                let mut seen = crate::DetHashSet::default();
                let endpoints: Vec<SolutionTerm<D::Id>> = left
                    .rows
                    .iter()
                    .filter_map(|row| row[c])
                    .filter(|term| seen.insert(*term))
                    .collect();
                EndpointBinding::Endpoints(endpoints.into())
            }
            // An enclosing operator already lists this variable's endpoints, and this
            // operand is in a direct position of that operator's right side (it would not
            // have listed them otherwise), so its rows reach that operator's join on the
            // variable: its list stays the one in force.
            _ if matches!(
                binding_for(&ctx.endpoint_frames[..depth], variable),
                Some(EndpointBinding::Endpoints(_))
            ) =>
            {
                continue;
            }
            Some(_) => EndpointBinding::PartlyUnbound,
            None => continue,
        };
        ctx.endpoint_frames.push(EndpointFrame {
            variable: variable.clone(),
            binding,
            role,
        });
    }
    let evaluated = eval_evaluated(right, ctx);
    ctx.endpoint_frames.truncate(depth);
    evaluated
}

/// One partition of a `MINUS`'s left solutions: the positions of the solutions that bind
/// every partitioning variable to `endpoints`, in left order.
pub(crate) struct MinusPartition<I> {
    /// Each partitioning variable with the term this partition binds it to.
    endpoints: Vec<(Variable, SolutionTerm<I>)>,
    /// The positions, in `left`, of the solutions in this partition.
    pub(crate) rows: Vec<usize>,
}

/// Partition `left`, the left solutions of `left MINUS right`, by the endpoints of the
/// variable-endpoint `SERVICE` clauses served in `right` — or `None` when there are none
/// to partition by, and `right` is evaluated once, as [`eval_right_operand`] evaluates
/// it.
///
/// A variable is partitioned by when every left solution binds it. One that some
/// solution leaves unbound is not an endpoint list at all, and the `SERVICE` is refused
/// where it is evaluated, so the whole operand takes the unpartitioned route; one an
/// enclosing operator already lists endpoints for keeps that list, as in
/// [`eval_right_operand`].
pub(crate) fn minus_partitions<D: DatasetView + Sync>(
    left: &SolutionSeq<D::Id>,
    right: &GraphPattern,
    ctx: &EvalCtx<'_, D>,
) -> Option<Vec<MinusPartition<D::Id>>> {
    let EndpointScan::Present(index) = ctx.endpoint_scan() else {
        return None;
    };
    let served = served_in(right, index, ctx);
    let mut columns: Vec<(Variable, usize)> = Vec::new();
    for variable in served.iter() {
        match left.schema.index_of(variable) {
            Some(c) if left.rows.iter().all(|row| row[c].is_some()) => {
                columns.push((variable.clone(), c));
            }
            _ if matches!(
                binding_for(&ctx.endpoint_frames, variable),
                Some(EndpointBinding::Endpoints(_))
            ) => {}
            Some(_) => return None,
            None => {}
        }
    }
    if columns.is_empty() {
        return None;
    }
    let mut partitions: Vec<MinusPartition<D::Id>> = Vec::new();
    let mut by_key: crate::DetHashMap<Vec<SolutionTerm<D::Id>>, usize> =
        crate::DetHashMap::default();
    for (position, row) in left.rows.iter().enumerate() {
        let key: Vec<SolutionTerm<D::Id>> = columns.iter().filter_map(|&(_, c)| row[c]).collect();
        let slot = *by_key.entry(key).or_insert_with_key(|key| {
            partitions.push(MinusPartition {
                endpoints: columns
                    .iter()
                    .zip(key)
                    .map(|((variable, _), term)| (variable.clone(), *term))
                    .collect(),
                rows: Vec::new(),
            });
            partitions.len() - 1
        });
        partitions[slot].rows.push(position);
    }
    Some(partitions)
}

/// Evaluate `right` for one [`MinusPartition`]: every partitioning variable's clause
/// invokes only the partition's endpoint, and a silenced invocation contributes Ω0.
///
/// # Errors
///
/// Whatever evaluating `right` raises.
pub(crate) fn eval_minus_partition<D: DatasetView + Sync>(
    partition: &MinusPartition<D::Id>,
    right: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let depth = ctx.endpoint_frames.len();
    for (variable, endpoint) in &partition.endpoints {
        ctx.endpoint_frames.push(EndpointFrame {
            variable: variable.clone(),
            binding: EndpointBinding::Endpoints(Arc::from([*endpoint])),
            role: FrameRole::Minus,
        });
    }
    let evaluated = eval_evaluated(right, ctx);
    ctx.endpoint_frames.truncate(depth);
    evaluated
}

/// Whether a group join `left ⋈ right` should take the endpoints of the variable-endpoint
/// `SERVICE` clauses in a direct position of `left` from `right`: when `left` holds one
/// and `right` holds none (a right operand that needs the left's endpoints keeps the
/// left-bound order), and some such clause's variable is one `right` may bind.
///
/// A join is commutative, and the exactness argument of the left-bound case holds with
/// the sides exchanged: every row the clause produces carries `?e`, every right row
/// binds `?e` (a right operand that leaves it unbound in some solution is refused, as a
/// left one is), so a row from an endpoint the right rows do not name joins none of them.
/// Only the inner join commutes: an `OPTIONAL` or `MINUS` keeps its endpoints on the
/// left. A clause an enclosing operator already lists endpoints for keeps that list, and
/// the order it had, so no endpoint outside it is asked. When no enclosing operator and nothing on the right binds the variable, the
/// clause is refused where it is evaluated, as before — the order changes nothing then —
/// so this is decided on the patterns alone.
pub(crate) fn binds_left_endpoints<D: DatasetView + Sync>(
    left: &GraphPattern,
    right: &GraphPattern,
    ctx: &EvalCtx<'_, D>,
) -> bool {
    let EndpointScan::Present(index) = ctx.endpoint_scan() else {
        return false;
    };
    let served = served_in(left, index, ctx);
    if served.is_empty() || !served_in(right, index, ctx).is_empty() {
        return false;
    }
    if served.iter().any(|variable| {
        matches!(
            binding_for(&ctx.endpoint_frames, variable),
            Some(EndpointBinding::Endpoints(_))
        )
    }) {
        return false;
    }
    let mut mentioned = crate::DetHashSet::default();
    crate::expr::pattern_all_vars(right, &mut mentioned);
    served.iter().any(|variable| mentioned.contains(variable))
}

/// Refuse a `LATERAL` whose right operand holds a variable-endpoint `SERVICE ?v` that
/// some left solution cannot name an endpoint for — before the right operand is
/// evaluated for any left solution, so no request of the clause goes out first.
///
/// A `LATERAL` (which `P . SERVICE ?v { … }` is parsed into) evaluates its right operand
/// once per left solution with that solution substituted in. A solution that binds `?v`
/// to an IRI resolves the clause; one that binds it to a literal or a blank node names no
/// endpoint; one that leaves it unbound leaves the clause to the endpoints an enclosing
/// operator lists, and is refused when none does. Found one solution at a time, the
/// refusal came after the requests of the solutions before it, so whether a request —
/// credentials included — went out for a query that is refused depended on row order.
/// This finds it first, for the clauses whose variable nothing else in the right operand
/// mentions ([`crate::expr::pattern_vars_outside`]), since only there is a left solution
/// the one thing that can bind it: a clause whose variable the right operand may bind
/// itself is left to its own evaluation. Every refusal made here is the one the
/// per-solution evaluation would have reached, and a query it admits evaluates exactly as
/// before.
///
/// A value that is not an IRI is refused here only when no clause of the variable is
/// `SILENT`: under `SILENT` it is an invocation that fails, and the substituted clause
/// answers it with Ω0 and a silenced record (see `crate::expr::substitute_pattern`).
///
/// # Errors
///
/// [`EvalError::Unsupported`] naming the value when a left solution binds `?v` to a term
/// that is not an IRI and the clause is not `SILENT` ([`non_iri_endpoint`]), and when
/// one leaves it unbound and no enclosing operator lists its endpoints
/// ([`unbound_endpoint`]), `SILENT` or not.
pub(crate) fn admit_lateral_endpoints<D: DatasetView + Sync>(
    left: &SolutionSeq<D::Id>,
    right: &GraphPattern,
    ctx: &EvalCtx<'_, D>,
) -> Result<(), EvalError> {
    if ctx.endpoint_scan().is_absent() || left.rows.is_empty() {
        return Ok(());
    }
    let mut uses: Vec<(Variable, bool)> = Vec::new();
    lateral_endpoint_uses(right, &mut uses);
    uses.retain(|(variable, _)| {
        let mut outside = crate::DetHashSet::default();
        crate::expr::pattern_vars_outside(right, Some(variable), &mut outside);
        !outside.contains(variable)
    });
    for (variable, silent) in uses {
        let column = left.schema.index_of(&variable);
        let mut unbound = 0_usize;
        for row in &left.rows {
            match column.and_then(|c| row[c]) {
                None => unbound += 1,
                Some(term) if !silent => {
                    let value = ctx.scratch.value_of(ctx.dataset, term);
                    if !matches!(value, TermValue::Iri(_)) {
                        return Err(non_iri_endpoint(&variable, &value));
                    }
                }
                Some(_) => {}
            }
        }
        if unbound == 0 {
            continue;
        }
        match binding_for(&ctx.endpoint_frames, &variable) {
            Some(EndpointBinding::Endpoints(_)) => {}
            binding => {
                let cause = if unbound < left.rows.len()
                    || matches!(binding, Some(EndpointBinding::PartlyUnbound))
                {
                    "some solutions of the pattern before it leave it unbound"
                } else {
                    "it is not bound to an IRI where the SERVICE is evaluated"
                };
                return Err(unbound_endpoint(&variable, cause));
            }
        }
    }
    Ok(())
}

/// Every variable-endpoint `SERVICE ?v` a per-solution evaluation of `pattern` reaches,
/// each variable once with whether every one of its clauses is `SILENT`: not inside a
/// `SERVICE` body (forwarded, never evaluated here) nor an expression, and not below a
/// sub-`SELECT` that does not project `?v` (a different `?v`, which no substitution
/// reaches).
///
/// A loop over an explicit work list, popping each node's children in written order, so
/// `uses` fills in the order the clauses are written and a right operand of any depth
/// costs heap and never stack.
fn lateral_endpoint_uses(pattern: &GraphPattern, uses: &mut Vec<(Variable, bool)>) {
    /// One step of the walk.
    enum Step<'a> {
        /// A pattern to walk.
        Pattern(&'a GraphPattern),
        /// A sub-`SELECT` whose inner pattern is walked: of the uses the walk added
        /// below it — from position `first` on — keep those it projects.
        LeaveProject(&'a [Variable], usize),
    }
    let mut pending = vec![Step::Pattern(pattern)];
    while let Some(step) = pending.pop() {
        match step {
            Step::Pattern(GraphPattern::Service { name, silent, .. }) => {
                if let NamedNodePattern::Variable(variable) = name {
                    if let Some((_, all_silent)) =
                        uses.iter_mut().find(|(seen, _)| seen == variable)
                    {
                        *all_silent &= *silent;
                    } else {
                        uses.push((variable.clone(), *silent));
                    }
                }
            }
            Step::Pattern(GraphPattern::Project { inner, variables }) => {
                pending.push(Step::LeaveProject(variables, uses.len()));
                pending.push(Step::Pattern(inner));
            }
            Step::Pattern(pattern) => {
                let first = pending.len();
                crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
                    if let PatternPart::Child(child, _) = part {
                        pending.push(Step::Pattern(child));
                    }
                    false
                });
                pending[first..].reverse();
            }
            Step::LeaveProject(variables, first) => {
                let mut kept = first;
                for index in first..uses.len() {
                    if variables.contains(&uses[index].0) {
                        uses.swap(kept, index);
                        kept += 1;
                    }
                }
                uses.truncate(kept);
            }
        }
    }
}

/// Evaluate a sub-`SELECT`'s `inner` with every endpoint variable it does not project
/// hidden, so a `SERVICE ?v` below it — a different `?v` — is never answered over the
/// outer operator's endpoints.
///
/// # Errors
///
/// Whatever evaluating `inner` raises.
pub(crate) fn eval_projected<D: DatasetView + Sync>(
    inner: &GraphPattern,
    variables: &[Variable],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    if ctx.endpoint_frames.is_empty() {
        return eval_evaluated(inner, ctx);
    }
    let depth = ctx.endpoint_frames.len();
    let hidden: Vec<Variable> = ctx.endpoint_frames[..depth]
        .iter()
        .filter(|frame| !variables.contains(&frame.variable))
        .map(|frame| frame.variable.clone())
        .collect();
    for variable in hidden {
        ctx.endpoint_frames.push(EndpointFrame {
            variable,
            binding: EndpointBinding::Hidden,
            role: FrameRole::Join,
        });
    }
    let evaluated = eval_evaluated(inner, ctx);
    ctx.endpoint_frames.truncate(depth);
    evaluated
}

/// Evaluate `SERVICE ?variable { … }` (`node`) over the endpoints the enclosing operator
/// lists, or refuse it when none does.
///
/// # Errors
///
/// [`EvalError::Unsupported`] when no enclosing solution names an endpoint (`SILENT` or
/// not) and, without `SILENT`, when the clause's variable is bound to a term that is not
/// an IRI; and whatever each endpoint's invocation raises.
pub(crate) fn eval_variable_endpoint<D: DatasetView + Sync>(
    node: &GraphPattern,
    variable: &Variable,
    silent: bool,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    match frame_for(&ctx.endpoint_frames, variable) {
        Some(EndpointFrame {
            binding: EndpointBinding::Endpoints(endpoints),
            role,
            ..
        }) => {
            let endpoints = Arc::clone(endpoints);
            let role = *role;
            eval_over_endpoints(node, variable, &endpoints, silent, role, ctx)
        }
        Some(EndpointFrame {
            binding: EndpointBinding::PartlyUnbound,
            ..
        }) => Err(unbound_endpoint(
            variable,
            "some solutions of the pattern before it leave it unbound",
        )),
        Some(EndpointFrame {
            binding: EndpointBinding::Hidden,
            ..
        })
        | None => Err(unbound_endpoint(
            variable,
            "it is not bound to an IRI where the SERVICE is evaluated",
        )),
    }
}

/// The refusal for a `SERVICE ?variable` no solution names an endpoint for.
pub(crate) fn unbound_endpoint(variable: &Variable, cause: &str) -> EvalError {
    let v = variable.as_str();
    EvalError::unsupported(format!(
        "SERVICE ?{v} with no endpoint: ?{v} names the endpoint, but {cause}, so there is \
         no endpoint to send the request to. A variable endpoint is evaluated once per \
         distinct IRI ?{v} is bound to, wherever ?{v} is bound in every solution that \
         reaches the SERVICE: by a pattern earlier in the same group (a triple pattern, \
         VALUES, BIND or LATERAL), by the left side of the OPTIONAL, MINUS or group join \
         whose right side holds the SERVICE, or by the other side of the group join that \
         holds it — but not through a further OPTIONAL or MINUS \
         right side, an EXISTS, a LIMIT/OFFSET, an aggregate that does not group by ?{v}, \
         or a sub-SELECT that does not project ?{v}. Bind ?{v} before the SERVICE, e.g. \
         `?s <p> ?{v} . SERVICE ?{v} {{ … }}` or `?s <p> ?{v} LATERAL {{ SERVICE ?{v} {{ … }} \
         }}`. SILENT does not change this: no invocation is made, so there is no failed \
         invocation for it to absorb"
    ))
}

/// The error for a `SERVICE ?variable` whose variable is bound to a term that is not an
/// IRI: it names no endpoint, so there is nothing to send the request to. Under `SILENT`
/// the invocation is silenced with this message instead.
pub(crate) fn non_iri_endpoint(variable: &Variable, value: &TermValue) -> EvalError {
    let v = variable.as_str();
    EvalError::unsupported(format!(
        "SERVICE ?{v}: ?{v} is bound to {}, which is not an IRI, so there is no endpoint to \
         send the request to",
        describe_non_iri(value)
    ))
}

/// A short description of a non-IRI term, for the error naming it.
fn describe_non_iri(value: &TermValue) -> String {
    match value {
        TermValue::Iri(iri) => format!("<{iri}>"),
        TermValue::Blank { .. } => "a blank node".to_owned(),
        TermValue::Literal { lexical_form, .. } => format!("the literal \"{lexical_form}\""),
        TermValue::Triple { .. } => "a triple term".to_owned(),
    }
}

/// `value` written out as the endpoint of a silenced-invocation record: an IRI as
/// itself, any other term in N-Triples form — a triple term as `<<( s p o )>>`, its
/// components in N-Triples form with an IRI bracketed, written over a work list so a
/// term nested to any depth costs no machine stack.
fn endpoint_text(value: &TermValue) -> String {
    let mut out = String::new();
    let mut first = true;
    let ControlFlow::Continue(()) =
        value.visit_terms_pre_post(|event: TermVisit<'_>| -> ControlFlow<Infallible> {
            match event {
                TermVisit::Open(_) => {
                    separate(&mut out, &mut first);
                    out.push_str("<<(");
                    first = false;
                }
                TermVisit::Close(_) => out.push_str(" )>>"),
                TermVisit::Leaf(term) => {
                    let nested = !first;
                    separate(&mut out, &mut first);
                    match term {
                        // The record's own endpoint is written bare; inside a triple
                        // term an IRI is bracketed.
                        TermValue::Iri(iri) if nested => write!(out, "<{iri}>"),
                        TermValue::Iri(iri) => write!(out, "{iri}"),
                        TermValue::Blank { label, .. } => write!(out, "_:{label}"),
                        TermValue::Literal {
                            lexical_form,
                            datatype,
                            language,
                            ..
                        } => match language {
                            Some(language) => write!(out, "\"{lexical_form}\"@{language}"),
                            None if datatype == XSD_STRING => write!(out, "\"{lexical_form}\""),
                            None => write!(out, "\"{lexical_form}\"^^<{datatype}>"),
                        },
                        TermValue::Triple { .. } => {
                            unreachable!("a triple term is opened and closed, never a leaf")
                        }
                    }
                    .expect("a String accepts text");
                }
            }
            ControlFlow::Continue(())
        });
    out
}

/// Put the space that separates a triple term's components before the next one:
/// nothing before the record's own endpoint, a space before everything after it.
fn separate(out: &mut String, first: &mut bool) {
    if *first {
        *first = false;
    } else {
        out.push(' ');
    }
}

/// The datatype a simple literal carries.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// What one endpoint contributed to the clause.
enum Block<I: purrdf_core::ViewTermId> {
    /// The endpoint's rows, each tagged with the endpoint.
    Rows(SolutionSeq<I>),
    /// A silenced invocation under `MINUS`: Ω0, untagged.
    Identity,
}

/// Each endpoint's own contribution, in endpoint order.
type EndpointBlocks<I> = Vec<(SolutionTerm<I>, Block<I>)>;

/// `⋃ { ?variable ↦ e } ⋈ Invocation(e, node's body)` over `endpoints`, in their order —
/// with a silenced invocation under a `MINUS` frame contributing Ω0 itself, untagged.
fn eval_over_endpoints<D: DatasetView + Sync>(
    node: &GraphPattern,
    variable: &Variable,
    endpoints: &[SolutionTerm<D::Id>],
    silent: bool,
    role: FrameRole,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let key = VarSchema::from_vars([variable.clone()]);
    let mut blocks: EndpointBlocks<D::Id> = Vec::with_capacity(endpoints.len());
    let mut stopped = None;
    // Without `SILENT`, a value that is not an IRI is refused before any request goes
    // out, so whether one does never depends on the order the endpoints are listed in.
    if !silent {
        for &endpoint in endpoints {
            let value = ctx.scratch.value_of(ctx.dataset, endpoint);
            if !matches!(value, TermValue::Iri(_)) {
                return Err(non_iri_endpoint(variable, &value));
            }
        }
    }
    for &endpoint in endpoints {
        let value = ctx.scratch.value_of(ctx.dataset, endpoint);
        let invocation = if matches!(value, TermValue::Iri(_)) {
            // The same substitution a `LATERAL` makes for one solution: the IRI becomes
            // the clause's endpoint and is injected into its body wherever the body names
            // it.
            let row = crate::expr::outer_bindings_for_substitution(&[Some(endpoint)], &key, ctx);
            let substituted = crate::expr::substitute_pattern(node, &row)?;
            let GraphPattern::Service {
                name: NamedNodePattern::NamedNode(iri),
                inner,
                silent: substituted_silent,
            } = substituted.as_ref()
            else {
                return Err(EvalError::internal(
                    "substituting an IRI endpoint into a variable-endpoint SERVICE did not \
                     resolve its endpoint",
                ));
            };
            crate::remote::invoke_service(iri.as_str(), inner, *substituted_silent, ctx)?
        } else {
            crate::remote::failed_invocation(
                &endpoint_text(&value),
                purrdf_core::SilencedKind::NotAnIri,
                non_iri_endpoint(variable, &value),
                silent,
            )?
        };
        match invocation {
            Invocation::Answered(Evaluated::Complete(seq)) => {
                blocks.push((endpoint, Block::Rows(seq)));
            }
            // Commit per endpoint: the block this endpoint was producing is discarded
            // whole, and every block before it is complete — a prefix of this clause's
            // ordered output.
            Invocation::Answered(Evaluated::Truncated(truncation)) => {
                stopped = Some(truncation.split().1);
                break;
            }
            Invocation::Silenced(record) => {
                ctx.record_silenced(record);
                blocks.push((
                    endpoint,
                    match role {
                        FrameRole::Join | FrameRole::Optional => {
                            Block::Rows(crate::remote::identity_seq())
                        }
                        FrameRole::Minus => Block::Identity,
                    },
                ));
            }
        }
    }

    let mut schema = VarSchema::from_vars([variable.clone()]);
    for (_, block) in &blocks {
        if let Block::Rows(block) = block {
            for v in block.schema.vars() {
                schema.push(v.clone());
            }
        }
    }
    let schema = Arc::new(schema);
    let width = schema.len();
    let mut rows: Vec<Solution<D::Id>> = Vec::new();
    for (endpoint, block) in &blocks {
        let block = match block {
            Block::Rows(block) => block,
            Block::Identity => {
                rows.push(smallvec::smallvec![None; width]);
                continue;
            }
        };
        let to_out: Vec<usize> = block
            .schema
            .vars()
            .iter()
            .map(|v| schema.index_of(v).unwrap_or(0))
            .collect();
        let own = block.schema.index_of(variable);
        for row in &block.rows {
            // A body that binds the endpoint variable itself was given the endpoint's IRI
            // by the substitution; a row binding it to anything else is not a solution
            // for this endpoint.
            if own.is_some_and(|j| row[j].is_some_and(|t| t != *endpoint)) {
                continue;
            }
            let mut out: Solution<D::Id> = smallvec::smallvec![None; width];
            out[0] = Some(*endpoint);
            for (j, cell) in row.iter().enumerate() {
                if cell.is_some() {
                    out[to_out[j]] = *cell;
                }
            }
            rows.push(out);
        }
    }
    let seq = SolutionSeq { schema, rows };
    Ok(match stopped {
        None => Evaluated::Complete(seq),
        Some(certificate) => Evaluated::Truncated(Truncation::new(seq, certificate)),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
    use purrdf_sparql_algebra::Variable;

    use crate::error::EvalError;
    use crate::eval::{EvalCtx, Outcome, evaluate_query, materialize_solutions};
    use crate::remote::{RemoteError, ResolvedBindings, ServiceRequest, ServiceResolver};

    const EX: &str = "http://example.org/";
    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

    /// `ex:g1 → ex:e1`, `ex:g2 → ex:e2`, `ex:g3 → ex:e1` (a repeated endpoint), `ex:g4 →
    /// ex:e3` (an endpoint that answers nothing) and `ex:g5 → ex:down` (an endpoint that
    /// fails), all through `ex:endpoint`; and `ex:g1 ex:other ex:o`.
    fn local() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let endpoint = b.intern_iri(&format!("{EX}endpoint"));
        for (g, e) in [
            ("g1", "e1"),
            ("g2", "e2"),
            ("g3", "e1"),
            ("g4", "e3"),
            ("g5", "down"),
        ] {
            let g = b.intern_iri(&format!("{EX}{g}"));
            let e = b.intern_iri(&format!("{EX}{e}"));
            b.push_quad(g, endpoint, e, None);
        }
        let g1 = b.intern_iri(&format!("{EX}g1"));
        let other = b.intern_iri(&format!("{EX}other"));
        let o = b.intern_iri(&format!("{EX}o"));
        b.push_quad(g1, other, o, None);
        b.freeze().expect("freeze")
    }

    /// Every endpoint answers `?x = "answer-from-<local name>"`, so a row joined to the
    /// wrong endpoint is visible in its value; `ex:e3` answers no rows and `ex:down` fails
    /// at the transport. Every request is recorded, in order.
    #[derive(Debug, Default)]
    struct Endpoints {
        requests: Mutex<Vec<String>>,
    }

    impl Endpoints {
        fn requests(&self) -> Vec<String> {
            self.requests.lock().expect("lock").clone()
        }
    }

    impl ServiceResolver for Endpoints {
        fn resolve(&self, request: ServiceRequest<'_>) -> Result<ResolvedBindings, RemoteError> {
            self.requests
                .lock()
                .expect("lock")
                .push(request.endpoint.to_owned());
            let name = request
                .endpoint
                .strip_prefix(EX)
                .unwrap_or(request.endpoint);
            if name == "down" {
                return Err(RemoteError::Transport("connection refused".to_owned()));
            }
            let rows = if name == "e3" {
                Vec::new()
            } else {
                vec![vec![Some(TermValue::Literal {
                    lexical_form: format!("answer-from-{name}"),
                    datatype: XSD_STRING.to_owned(),
                    language: None,
                    direction: None,
                })]]
            };
            Ok(ResolvedBindings {
                variables: vec![Variable::new("x")],
                rows,
                cell_limit_exceeded_at: None,
            })
        }
    }

    /// Run `query` over [`local`] with `source`, rendering each row as `name=value` cells
    /// joined by `&` (unbound names omitted, local names for IRIs), sorted.
    fn run(source: &Endpoints, query: &str) -> Result<Vec<String>, EvalError> {
        let ds = local();
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(query)
            .expect("parse");
        let mut ctx = EvalCtx::new(&ds).with_remote(source);
        let Outcome::Solutions(seq) = evaluate_query(&parsed, &mut ctx)? else {
            panic!("a SELECT answers solutions");
        };
        let (variables, rows) = materialize_solutions(&seq, &ctx);
        let mut out: Vec<String> = rows
            .iter()
            .map(|row| {
                let mut cells: Vec<String> = variables
                    .iter()
                    .zip(row)
                    .filter_map(|(v, cell)| {
                        cell.as_ref().map(|value| {
                            let text = match value {
                                TermValue::Iri(iri) => {
                                    iri.strip_prefix(EX).unwrap_or(iri).to_owned()
                                }
                                TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
                                other => format!("{other:?}"),
                            };
                            format!("{v}={text}")
                        })
                    })
                    .collect();
                cells.sort();
                cells.join("&")
            })
            .collect();
        out.sort();
        Ok(out)
    }

    /// Rendered rows, and each silenced invocation as `(endpoint, kind label)`.
    type Recorded = (Vec<String>, Vec<(String, &'static str)>);

    /// [`run`] under a metering governor, with the silenced invocations the evidence
    /// records, each as its endpoint (local name for an IRI) and kind label.
    fn run_recorded(source: &Endpoints, query: &str) -> Result<Recorded, EvalError> {
        let ds = local();
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(query)
            .expect("parse");
        let state = Arc::new(crate::governor::GovernorState::new(
            &crate::governor::QueryGovernors::METERED,
        ));
        let mut ctx = EvalCtx::new(&ds)
            .with_remote(source)
            .with_governors(Arc::clone(&state));
        let Outcome::Solutions(seq) = evaluate_query(&parsed, &mut ctx)? else {
            panic!("a SELECT answers solutions");
        };
        let (variables, rows) = materialize_solutions(&seq, &ctx);
        let mut out: Vec<String> = rows
            .iter()
            .map(|row| {
                let mut cells: Vec<String> = variables
                    .iter()
                    .zip(row)
                    .filter_map(|(v, cell)| {
                        cell.as_ref().map(|value| {
                            let text = match value {
                                TermValue::Iri(iri) => {
                                    iri.strip_prefix(EX).unwrap_or(iri).to_owned()
                                }
                                TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
                                other => format!("{other:?}"),
                            };
                            format!("{v}={text}")
                        })
                    })
                    .collect();
                cells.sort();
                cells.join("&")
            })
            .collect();
        out.sort();
        let silenced = state
            .evidence()
            .silenced()
            .iter()
            .map(|record| {
                let name = record.target.name();
                (
                    name.strip_prefix(EX).unwrap_or(name).to_owned(),
                    record.kind.label(),
                )
            })
            .collect();
        Ok((out, silenced))
    }

    fn q(pattern: &str) -> String {
        use std::fmt::Write as _;

        // `ex:name` → `<http://example.org/name>`, one token at a time.
        let mut out = String::new();
        for token in pattern.split(' ') {
            if let Some(local) = token.strip_prefix("ex:") {
                let _ = write!(out, "<{EX}{local}>");
            } else {
                out.push_str(token);
            }
            out.push(' ');
        }
        format!("SELECT ?g ?e ?x WHERE {{ {out}}}")
    }

    fn requested(source: &Endpoints) -> Vec<String> {
        let mut endpoints: Vec<String> = source
            .requests()
            .into_iter()
            .map(|e| e.strip_prefix(EX).unwrap_or(&e).to_owned())
            .collect();
        endpoints.sort();
        endpoints
    }

    /// Every endpoint but the failing one, bound by the left side.
    const BOUND: &str = "VALUES ?e { ex:e1 ex:e2 ex:e3 } ?g ex:endpoint ?e";

    #[test]
    fn optional_answers_per_endpoint_and_keeps_a_left_row_its_endpoint_answers_nothing_for() {
        let source = Endpoints::default();
        let rows = run(
            &source,
            &q(&format!("{BOUND} OPTIONAL {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect("an endpoint bound by the OPTIONAL's left side is evaluated");
        assert_eq!(
            rows,
            [
                "e=e1&g=g1&x=answer-from-e1",
                "e=e1&g=g3&x=answer-from-e1",
                "e=e2&g=g2&x=answer-from-e2",
                "e=e3&g=g4",
            ],
            "each left row joins its own endpoint's answer; ex:g4's endpoint answered \
             nothing, so the OPTIONAL keeps its left bindings"
        );
        assert_eq!(
            requested(&source),
            ["e1", "e2", "e3"],
            "one request per distinct endpoint: ex:e1 is bound twice and asked once"
        );
    }

    #[test]
    fn a_group_join_answers_per_endpoint() {
        let source = Endpoints::default();
        let rows = run(
            &source,
            &q(&format!("{{ {BOUND} }} {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect("an endpoint bound by the group join's left side is evaluated");
        assert_eq!(
            rows,
            [
                "e=e1&g=g1&x=answer-from-e1",
                "e=e1&g=g3&x=answer-from-e1",
                "e=e2&g=g2&x=answer-from-e2",
            ],
            "the inner join drops ex:g4, whose endpoint answered nothing"
        );
        assert_eq!(requested(&source), ["e1", "e2", "e3"]);

        // The same join written flat is the LATERAL route: the same rows.
        let flat = Endpoints::default();
        assert_eq!(
            run(&flat, &q(&format!("{BOUND} SERVICE ?e {{ ?s ?p ?x }}"))).expect("flat"),
            rows,
            "the group join and the flat join are the same join"
        );
    }

    #[test]
    fn minus_removes_the_rows_whose_endpoint_has_a_compatible_answer() {
        let source = Endpoints::default();
        let rows = run(
            &source,
            &q(&format!("{BOUND} MINUS {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect("an endpoint bound by the MINUS's left side is evaluated");
        assert_eq!(
            rows,
            ["e=e3&g=g4"],
            "every row whose endpoint answered is removed; ex:g4's answered nothing"
        );
        assert_eq!(requested(&source), ["e1", "e2", "e3"]);

        // The FILTER NOT EXISTS reading substitutes each row instead: the same rows.
        let exists = Endpoints::default();
        assert_eq!(
            run(
                &exists,
                &q(&format!(
                    "{BOUND} FILTER NOT EXISTS {{ SERVICE ?e {{ ?s ?p ?x }} }}"
                ))
            )
            .expect("not exists"),
            rows
        );
    }

    #[test]
    fn a_silent_endpoint_failure_is_the_identity_for_that_endpoint_only() {
        let all = "?g ex:endpoint ?e";
        let source = Endpoints::default();
        let rows = run(
            &source,
            &q(&format!(
                "{all} OPTIONAL {{ SERVICE SILENT ?e {{ ?s ?p ?x }} }}"
            )),
        )
        .expect("SILENT swallows the failing endpoint");
        assert_eq!(
            rows,
            [
                "e=down&g=g5",
                "e=e1&g=g1&x=answer-from-e1",
                "e=e1&g=g3&x=answer-from-e1",
                "e=e2&g=g2&x=answer-from-e2",
                "e=e3&g=g4",
            ]
        );
        assert_eq!(requested(&source), ["down", "e1", "e2", "e3"]);

        // In a group join the failing endpoint keeps its own left row and nobody else's:
        // an identity row without the endpoint binding would pad every left row.
        let join = Endpoints::default();
        assert_eq!(
            run(
                &join,
                &q(&format!(
                    "{{ {all} }} {{ SERVICE SILENT ?e {{ ?s ?p ?x }} }}"
                ))
            )
            .expect("group join"),
            [
                "e=down&g=g5",
                "e=e1&g=g1&x=answer-from-e1",
                "e=e1&g=g3&x=answer-from-e1",
                "e=e2&g=g2&x=answer-from-e2",
            ]
        );

        // Without SILENT the same failure is the query's failure.
        let loud = Endpoints::default();
        let error = run(
            &loud,
            &q(&format!("{all} OPTIONAL {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect_err("a failing endpoint fails a non-silent SERVICE");
        assert!(
            matches!(&error, EvalError::Remote(m) if m.contains("http://example.org/down")),
            "{error}"
        );
    }

    #[test]
    fn a_silenced_endpoint_under_minus_removes_none_of_its_rows() {
        // `ex:down` fails. Under SILENT its invocation is Ω0, which binds nothing and so
        // removes nothing: its left row survives, and the failure is recorded.
        let only_down =
            "VALUES ?e { ex:down } ?g ex:endpoint ?e MINUS { SERVICE SILENT ?e { ?s ?p ?x } }";
        let source = Endpoints::default();
        assert_eq!(
            run_recorded(&source, &q(only_down)).expect("silenced"),
            (
                vec!["e=down&g=g5".to_owned()],
                vec![("down".to_owned(), "transport")]
            )
        );
        assert_eq!(requested(&source), ["down"]);
        // The neighbour: the same shape over an endpoint that answers a row compatible
        // with its left rows removes exactly those rows (both of `ex:e1`'s).
        let only_e1 = only_down.replace("ex:down", "ex:e1");
        let source = Endpoints::default();
        assert_eq!(
            run_recorded(&source, &q(&only_e1)).expect("answered"),
            (Vec::<String>::new(), Vec::new())
        );
        assert_eq!(requested(&source), ["e1"]);
        // Without SILENT the failing endpoint is an error.
        let source = Endpoints::default();
        let error = run(&source, &q(&only_down.replace("SILENT ", ""))).expect_err("fails");
        assert!(matches!(error, EvalError::Remote(_)), "{error}");
    }

    #[test]
    fn under_minus_a_failing_endpoint_keeps_its_rows_while_answering_ones_remove_theirs() {
        // Every endpoint: `ex:e1` and `ex:e2` answer, so their rows go; `ex:e3` answers
        // nothing and `ex:down` fails under SILENT, so theirs stay. One request each.
        let source = Endpoints::default();
        let (rows, silenced) = run_recorded(
            &source,
            &q("?g ex:endpoint ?e MINUS { SERVICE SILENT ?e { ?s ?p ?x } }"),
        )
        .expect("evaluates");
        assert_eq!(rows, ["e=down&g=g5", "e=e3&g=g4"]);
        assert_eq!(silenced, [("down".to_owned(), "transport")]);
        assert_eq!(requested(&source), ["down", "e1", "e2", "e3"]);
        // The rows keep the left side's order: `ex:g1`..`ex:g5` in data order, less the
        // removed ones, is what an order-preserving merge of the partitions gives.
        let source = Endpoints::default();
        let ds = local();
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(&format!(
                "SELECT ?g WHERE {{ VALUES ?g {{ <{EX}g5> <{EX}g1> <{EX}g4> <{EX}g3> }} \
                 ?g <{EX}endpoint> ?e MINUS {{ SERVICE SILENT ?e {{ ?s ?p ?x }} }} }}"
            ))
            .expect("parse");
        let mut ctx = EvalCtx::new(&ds).with_remote(&source);
        let Outcome::Solutions(seq) = evaluate_query(&parsed, &mut ctx).expect("evaluates") else {
            panic!("a SELECT answers solutions");
        };
        let (_, ordered) = materialize_solutions(&seq, &ctx);
        let ordered: Vec<String> = ordered
            .iter()
            .map(|row| match &row[0] {
                Some(TermValue::Iri(iri)) => iri.strip_prefix(EX).unwrap_or(iri).to_owned(),
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(ordered, ["g5", "g4"]);
    }

    #[test]
    fn the_served_index_agrees_with_the_walk_on_every_operand() {
        use purrdf_sparql_algebra::GraphPattern;

        fn operands<'p>(pattern: &'p GraphPattern, out: &mut Vec<&'p GraphPattern>) {
            match pattern {
                GraphPattern::Join { left, right } => {
                    out.push(left);
                    out.push(right);
                }
                GraphPattern::LeftJoin { right, .. } | GraphPattern::Minus { right, .. } => {
                    out.push(right);
                }
                _ => {}
            }
            crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
                match part {
                    crate::governor::soundness::PatternPart::Child(child, _) => {
                        operands(child, out);
                    }
                    crate::governor::soundness::PatternPart::Expression(expr) => {
                        crate::governor::soundness::visit_expression_parts(expr, &mut |p| {
                            if let crate::governor::soundness::ExpressionPart::Exists(body) = p {
                                operands(body, out);
                            }
                            false
                        });
                    }
                }
                false
            });
        }

        for body in [
            "{ ?g ex:endpoint ?e } { SERVICE ?e { ?s ?p ?x } }",
            "?g ex:endpoint ?e OPTIONAL { SERVICE ?e { ?s ?p ?x } } MINUS { SERVICE ?e { ?a ?b ?c } }",
            "{ SELECT ?e WHERE { { ?g ex:endpoint ?e } { SERVICE ?e { ?s ?p ?x } } } } { ?g ex:other ?o }",
            "{ ?g ex:endpoint ?e } { { SERVICE ?e { ?s ?p ?x } } UNION { ?s ?p ?x } }",
            "{ ?g ex:endpoint ?e } { SELECT ?x WHERE { SERVICE ?e { ?s ?p ?x } } }",
            "{ ?g ex:endpoint ?e } { SELECT ?e (COUNT(*) AS ?n) WHERE { SERVICE ?e { ?s ?p ?x } } GROUP BY ?e }",
            "{ ?g ex:endpoint ?e } { SELECT ?x WHERE { SERVICE ?e { ?s ?p ?x } } LIMIT 1 }",
            "{ ?g ex:endpoint ?e } { SERVICE ?e { ?s ?p ?x } FILTER EXISTS { { ?a ?b ?c } { SERVICE ?e { ?a ?b ?c } } } }",
            "{ ?g ex:endpoint ?e } { ?g ex:other ?o OPTIONAL { SERVICE ?e { ?s ?p ?x } } }",
        ] {
            let query = q(body);
            let parsed = purrdf_sparql_algebra::SparqlParser::new()
                .parse_query(&query)
                .expect("parse");
            let pattern = crate::eval::query_pattern(&parsed);
            let tree = crate::plan::Tree::build(pattern);
            let super::EndpointScan::Present(index) = tree.shape().endpoints() else {
                panic!("{body}: the query has a variable endpoint");
            };
            let mut all = Vec::new();
            operands(pattern, &mut all);
            assert!(!all.is_empty(), "{body}");
            for operand in all {
                let walked = super::served_endpoint_variables(operand, None);
                let indexed = tree
                    .shape()
                    .node_of(operand)
                    .and_then(|operand| index.served_at(operand))
                    .unwrap_or_else(|| panic!("{body}: an operand the scan did not index"));
                assert_eq!(indexed.as_ref(), walked.as_slice(), "{body}");
            }
        }
        // And a query with none is absent, with no index built.
        let parsed = purrdf_sparql_algebra::SparqlParser::new()
            .parse_query(&q("{ ?g ex:endpoint ?e } { SERVICE ex:e1 { ?s ?p ?x } }"))
            .expect("parse");
        assert!(
            crate::plan::Tree::build(crate::eval::query_pattern(&parsed))
                .shape()
                .endpoints()
                .is_absent()
        );
    }

    #[test]
    fn an_endpoint_no_solution_binds_is_refused_even_under_silent() {
        for shape in [
            "SERVICE ?e { ?s ?p ?x }",
            "SERVICE SILENT ?e { ?s ?p ?x }",
            "?g ex:other ?o OPTIONAL { SERVICE SILENT ?e { ?s ?p ?x } }",
            "?g ex:other ?o MINUS { SERVICE SILENT ?e { ?s ?p ?x } }",
            "{ ?g ex:other ?o } { SERVICE SILENT ?e { ?s ?p ?x } }",
        ] {
            let source = Endpoints::default();
            let error = run(&source, &q(shape)).expect_err(shape);
            assert!(
                matches!(&error, EvalError::Unsupported { what, .. }
                    if what.contains("SERVICE ?e with no endpoint")
                        && what.contains("LATERAL")),
                "{shape}: {error}"
            );
            assert_eq!(
                source.requests(),
                Vec::<String>::new(),
                "{shape}: nothing was asked"
            );
        }
        // The bound neighbours answer from the endpoint under SILENT: a swallowed refusal
        // would have left `ex:g1` alone, with no `?x` and no request.
        for shape in [
            "?g ex:other ?o . ?g ex:endpoint ?e OPTIONAL { SERVICE SILENT ?e { ?s ?p ?x } }",
            "?g ex:other ?o . ?g ex:endpoint ?e SERVICE SILENT ?e { ?s ?p ?x }",
        ] {
            let source = Endpoints::default();
            assert_eq!(
                run(&source, &q(shape)).expect(shape),
                ["e=e1&g=g1&x=answer-from-e1"],
                "{shape}"
            );
            assert_eq!(requested(&source), ["e1"], "{shape}");
        }
    }

    #[test]
    fn a_left_side_that_leaves_the_endpoint_unbound_somewhere_is_refused() {
        let source = Endpoints::default();
        let error = run(
            &source,
            &q("{ ?g ex:endpoint ?e } UNION { ?g ex:other ?o } OPTIONAL { SERVICE ?e { ?s ?p ?x } }"),
        )
        .expect_err("a left row without an endpoint names none");
        assert!(
            matches!(&error, EvalError::Unsupported { what, .. }
                if what.contains("some solutions of the pattern before it leave it unbound")),
            "{error}"
        );
        assert_eq!(source.requests(), Vec::<String>::new());
        // The neighbour whose every left row binds it answers.
        let bound = Endpoints::default();
        assert_eq!(
            run(
                &bound,
                &q("{ ?g ex:endpoint ?e FILTER ( ?e = ex:e2 ) } UNION { ?g ex:endpoint ?e FILTER ( ?e = ex:e3 ) } OPTIONAL { SERVICE ?e { ?s ?p ?x } }")
            )
            .expect("bound in every row"),
            ["e=e2&g=g2&x=answer-from-e2", "e=e3&g=g4"]
        );
    }

    #[test]
    fn a_service_under_a_further_optional_or_an_unprojecting_subselect_is_refused() {
        for shape in [
            // Under a further OPTIONAL whose own left side does not bind ?e.
            "?g ex:endpoint ?e { ?g ex:other ?o OPTIONAL { SERVICE ?e { ?s ?p ?x } } }",
            // A sub-SELECT that does not project ?e: its ?e is a different variable.
            "?g ex:endpoint ?e { SELECT ?x { SERVICE ?e { ?s ?p ?x } } }",
        ] {
            let source = Endpoints::default();
            let error = run(&source, &q(shape)).expect_err(shape);
            assert!(
                matches!(&error, EvalError::Unsupported { what, .. }
                    if what.contains("SERVICE ?e with no endpoint")),
                "{shape}: {error}"
            );
            assert_eq!(source.requests(), Vec::<String>::new(), "{shape}");
        }
        // The neighbours: the OPTIONAL with ?e bound on its own left, and the sub-SELECT
        // that projects ?e.
        for shape in [
            "VALUES ?e { ex:e1 } ?g ex:endpoint ?e { ?g ex:other ?o . ?g ex:endpoint ?e OPTIONAL { SERVICE ?e { ?s ?p ?x } } }",
            "VALUES ?e { ex:e1 } ?g ex:endpoint ?e { SELECT ?e ?x { SERVICE ?e { ?s ?p ?x } } }",
        ] {
            let source = Endpoints::default();
            let rows = run(&source, &q(shape)).expect(shape);
            assert!(
                rows.contains(&"e=e1&g=g1&x=answer-from-e1".to_owned()),
                "{shape}: {rows:?}"
            );
            assert!(
                rows.iter().all(|row| !row.contains("answer-from-e2")),
                "{shape}: only ex:e1 is bound on the left: {rows:?}"
            );
        }
    }

    #[test]
    fn an_endpoint_bound_to_a_literal_is_an_endpoint_failure() {
        // SILENT: the single empty solution for that endpoint, on the endpoint-list route
        // (a VALUES or a pattern before the SERVICE) and the substitution route (BIND,
        // LATERAL), recorded as a value that is not an IRI, with no request.
        for shape in [
            "VALUES ?e { \"not-an-iri\" } OPTIONAL { SERVICE SILENT ?e { ?s ?p ?x } }",
            "VALUES ?e { \"not-an-iri\" } SERVICE SILENT ?e { ?s ?p ?x }",
            "BIND(\"not-an-iri\" AS ?e) SERVICE SILENT ?e { ?s ?p ?x }",
            "VALUES ?e { \"not-an-iri\" } LATERAL { SERVICE SILENT ?e { ?s ?p ?x } }",
        ] {
            let source = Endpoints::default();
            let (rows, silenced) = run_recorded(&source, &q(shape)).expect(shape);
            assert_eq!(rows, ["e=not-an-iri"], "{shape}");
            assert_eq!(
                silenced,
                [("\"not-an-iri\"".to_owned(), "not-an-iri")],
                "{shape}"
            );
            assert_eq!(source.requests(), Vec::<String>::new(), "{shape}");
            // Not SILENT: an error, and no request.
            let plain = shape.replace("SILENT ", "");
            let source = Endpoints::default();
            let error = run(&source, &q(&plain)).expect_err(&plain);
            assert!(
                error
                    .to_string()
                    .contains("?e is bound to the literal \"not-an-iri\", which is not an IRI"),
                "{plain}: {error}"
            );
            assert_eq!(source.requests(), Vec::<String>::new(), "{plain}");
        }
        // The neighbours: an IRI endpoint under SILENT answers, one request, nothing
        // silenced; and a listed endpoint that fails at the transport is the identity
        // under SILENT, for that endpoint alone, recorded as a transport failure.
        let source = Endpoints::default();
        assert_eq!(
            run_recorded(
                &source,
                &q("VALUES ?e { <http://example.org/e1> } SERVICE SILENT ?e { ?s ?p ?x }")
            )
            .expect("an IRI endpoint answers"),
            (vec!["e=e1&x=answer-from-e1".to_owned()], Vec::new())
        );
        assert_eq!(source.requests(), [format!("{EX}e1")]);
        let source = Endpoints::default();
        assert_eq!(
            run_recorded(
                &source,
                &q("VALUES ?e { <http://example.org/down> } SERVICE SILENT ?e { ?s ?p ?x }")
            )
            .expect("a failing endpoint is the identity under SILENT"),
            (
                vec!["e=down".to_owned()],
                vec![("down".to_owned(), "transport")]
            )
        );
        assert_eq!(source.requests(), [format!("{EX}down")]);
    }

    /// A left solution that names no endpoint — `?e` unbound, or bound to a literal — is
    /// refused before the clause asks any endpoint, whichever order the solutions come
    /// in: before, the solutions ahead of it had already sent their requests.
    #[test]
    fn a_lateral_endpoint_some_solution_cannot_name_is_refused_before_any_request() {
        for (shape, cause) in [
            (
                "VALUES ?g { ex:g1 ex:nobody } OPTIONAL { ?g ex:endpoint ?e } SERVICE ?e { ?s ?p ?x }",
                "some solutions of the pattern before it leave it unbound",
            ),
            (
                "VALUES ?g { ex:nobody ex:g1 } OPTIONAL { ?g ex:endpoint ?e } SERVICE ?e { ?s ?p ?x }",
                "some solutions of the pattern before it leave it unbound",
            ),
            (
                "VALUES ?g { ex:g1 ex:nobody } OPTIONAL { ?g ex:endpoint ?e } LATERAL { ?g ex:other ?o SERVICE ?e { ?s ?p ?x } }",
                "some solutions of the pattern before it leave it unbound",
            ),
            (
                "VALUES ?e { ex:e1 \"x\" } SERVICE ?e { ?s ?p ?x }",
                "?e is bound to the literal \"x\", which is not an IRI",
            ),
            (
                "VALUES ?e { \"x\" ex:e1 } SERVICE ?e { ?s ?p ?x }",
                "?e is bound to the literal \"x\", which is not an IRI",
            ),
        ] {
            let source = Endpoints::default();
            let error = run(&source, &q(shape)).expect_err(shape);
            assert!(error.to_string().contains(cause), "{shape}: {error}");
            assert_eq!(source.requests(), Vec::<String>::new(), "{shape}");
        }
        // The valid neighbours: every solution names an endpoint, and the clause answers
        // from each; a clause whose variable the right operand binds itself is left to
        // its own evaluation, and answers too.
        let source = Endpoints::default();
        assert_eq!(
            run(
                &source,
                &q("VALUES ?g { ex:g1 ex:g2 } OPTIONAL { ?g ex:endpoint ?e } SERVICE ?e { ?s ?p ?x }")
            )
            .expect("every solution names an endpoint"),
            ["e=e1&g=g1&x=answer-from-e1", "e=e2&g=g2&x=answer-from-e2"]
        );
        assert_eq!(requested(&source), ["e1", "e2"]);
        let source = Endpoints::default();
        assert_eq!(
            run(
                &source,
                &q("VALUES ?g { ex:g1 ex:nobody } LATERAL { ?g ex:endpoint ?e SERVICE ?e { ?s ?p ?x } }")
            )
            .expect("the right operand binds the endpoint itself"),
            ["e=e1&g=g1&x=answer-from-e1"]
        );
        assert_eq!(requested(&source), ["e1"]);
        // Under SILENT a literal is an invocation that fails: Ω0 for its solution, and
        // the IRI beside it is still asked.
        let source = Endpoints::default();
        assert_eq!(
            run_recorded(
                &source,
                &q("VALUES ?e { \"x\" ex:e1 } SERVICE SILENT ?e { ?s ?p ?x }")
            )
            .expect("a literal endpoint is silenced"),
            (
                vec!["e=e1&x=answer-from-e1".to_owned(), "e=x".to_owned()],
                vec![("\"x\"".to_owned(), "not-an-iri")]
            )
        );
        assert_eq!(requested(&source), ["e1"]);
    }
    /// A join is commutative: the endpoint bound on the RIGHT of a group join —
    /// `{ SERVICE ?e { … } ?g ex:endpoint ?e }`, `{ SERVICE ?e { … } VALUES ?e { … } }` —
    /// answers the rows the left-bound form answers, asking the same endpoints once each.
    #[test]
    fn an_endpoint_bound_on_the_right_of_a_group_join_answers_as_the_left_bound_form() {
        let left_bound = Endpoints::default();
        let expected = run(
            &left_bound,
            &q(&format!("{{ {BOUND} }} {{ SERVICE ?e {{ ?s ?p ?x }} }}")),
        )
        .expect("left-bound");
        for shape in [
            format!("{{ SERVICE ?e {{ ?s ?p ?x }} }} {{ {BOUND} }}"),
            format!("SERVICE ?e {{ ?s ?p ?x }} {{ {BOUND} }}"),
        ] {
            let source = Endpoints::default();
            assert_eq!(run(&source, &q(&shape)).expect(&shape), expected, "{shape}");
            assert_eq!(requested(&source), requested(&left_bound), "{shape}");
            assert_eq!(
                source.requests().len(),
                3,
                "{shape}: one request per endpoint"
            );
        }
        let source = Endpoints::default();
        assert_eq!(
            run(
                &source,
                &q("SERVICE ?e { ?s ?p ?x } VALUES ?e { ex:e2 ex:e1 ex:e2 }")
            )
            .expect("VALUES on the right"),
            [
                "e=e1&x=answer-from-e1",
                "e=e2&x=answer-from-e2",
                "e=e2&x=answer-from-e2",
            ]
        );
        assert_eq!(requested(&source), ["e1", "e2"]);
        // A bare triple pattern on the right, every endpoint including the failing one,
        // under SILENT: the rows and requests of the left-bound group join.
        let left_bound = Endpoints::default();
        let expected = run(
            &left_bound,
            &q("{ ?g ex:endpoint ?e } { SERVICE SILENT ?e { ?s ?p ?x } }"),
        )
        .expect("left-bound");
        let source = Endpoints::default();
        assert_eq!(
            run(
                &source,
                &q("SERVICE SILENT ?e { ?s ?p ?x } ?g ex:endpoint ?e")
            )
            .expect("a triple pattern on the right"),
            expected
        );
        assert_eq!(requested(&source), requested(&left_bound));
        assert_eq!(requested(&source), ["down", "e1", "e2", "e3"]);
        // SILENT per endpoint, as on the left: the failing endpoint's right row survives
        // with the identity.
        let source = Endpoints::default();
        assert_eq!(
            run(
                &source,
                &q("SERVICE SILENT ?e { ?s ?p ?x } VALUES ?e { ex:e1 ex:down }")
            )
            .expect("SILENT on the right"),
            ["e=down", "e=e1&x=answer-from-e1"]
        );
        assert_eq!(requested(&source), ["down", "e1"]);
    }

    /// What does not commute, or is not certainly bound, stays refused with no request:
    /// an endpoint bound on neither side, one an `OPTIONAL` on the right may leave
    /// unbound, and an `OPTIONAL` or `MINUS` whose left side is the `SERVICE`.
    #[test]
    fn an_endpoint_not_certainly_bound_on_the_right_stays_refused() {
        for shape in [
            "SERVICE ?e { ?s ?p ?x } ?g ex:other ?o",
            "SERVICE ?e { ?s ?p ?x } OPTIONAL { ?g ex:endpoint ?e }",
            "{ SERVICE ?e { ?s ?p ?x } } { VALUES ?g { ex:g1 ex:nobody } OPTIONAL { ?g ex:endpoint ?e } }",
            "SERVICE ?e { ?s ?p ?x } MINUS { ?g ex:endpoint ?e }",
        ] {
            let source = Endpoints::default();
            let error = run(&source, &q(shape)).expect_err(shape);
            assert!(
                error.to_string().contains("SERVICE ?e with no endpoint"),
                "{shape}: {error}"
            );
            assert_eq!(source.requests(), Vec::<String>::new(), "{shape}");
        }
    }
}

#[cfg(test)]
pub(crate) mod walk_tests {
    //! The loop-driven walks of this module against recursive references, over generated
    //! shapes and at a depth no thread stack holds.

    use std::sync::Arc;

    use purrdf_sparql_algebra::{
        AggregateExpression, AggregateFunction, Args, Chain, Child, Expression, Function,
        GraphPattern, NamedNode, NamedNodePattern, OrderExpression, PropertyPathExpression,
        TermPattern, TriplePattern, Variable,
    };

    use super::{EndpointUse, Occurrence, Scope, ServedIndex, indirect, merge, record};
    use crate::governor::soundness::{ExpressionPart, PatternPart};
    use crate::plan::NodeId;
    use crate::test_rng::splitmix64_next;

    const EX: &str = "http://example.org/";

    // ── The recursive references ────────────────────────────────────────────────────

    /// [`super::classify`], as a recursion over the pattern.
    fn classify_reference<'a>(
        pattern: &'a GraphPattern,
        direct: bool,
        scopes: &mut Vec<Scope<'a>>,
        uses: &mut Vec<EndpointUse>,
    ) {
        match pattern {
            GraphPattern::Bgp { .. }
            | GraphPattern::Path { .. }
            | GraphPattern::Values { .. }
            | GraphPattern::PropertyFunction(_) => {}
            GraphPattern::Service { name, .. } => {
                if let NamedNodePattern::Variable(variable) = name {
                    record(variable, direct, scopes, uses);
                }
            }
            GraphPattern::Join { left, right } | GraphPattern::Lateral { left, right } => {
                classify_reference(left, direct, scopes, uses);
                classify_reference(right, direct, scopes, uses);
            }
            GraphPattern::Union { arms } => {
                for arm in arms {
                    classify_reference(arm, direct, scopes, uses);
                }
            }
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => {
                classify_reference(left, direct, scopes, uses);
                classify_reference(right, false, scopes, uses);
                if let Some(expression) = expression {
                    classify_expression_reference(expression, scopes, uses);
                }
            }
            GraphPattern::Minus { left, right } => {
                classify_reference(left, direct, scopes, uses);
                classify_reference(right, false, scopes, uses);
            }
            GraphPattern::Filter { expr, inner } => {
                classify_reference(inner, direct, scopes, uses);
                classify_expression_reference(expr, scopes, uses);
            }
            GraphPattern::Extend {
                inner, expression, ..
            }
            | GraphPattern::Unfold {
                inner, expression, ..
            } => {
                classify_reference(inner, direct, scopes, uses);
                classify_expression_reference(expression, scopes, uses);
            }
            GraphPattern::Graph { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner } => classify_reference(inner, direct, scopes, uses),
            GraphPattern::OrderBy { inner, expression } => {
                classify_reference(inner, direct, scopes, uses);
                for key in expression {
                    classify_expression_reference(
                        crate::modifier::order_sort_key(key),
                        scopes,
                        uses,
                    );
                }
            }
            GraphPattern::Project { inner, variables } => {
                scopes.push(Scope::Project(variables));
                classify_reference(inner, direct, scopes, uses);
                scopes.pop();
            }
            GraphPattern::Slice {
                inner,
                start,
                length,
            } => {
                let identity = *start == 0 && length.is_none();
                classify_reference(inner, direct && identity, scopes, uses);
            }
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => {
                scopes.push(Scope::Group(variables));
                classify_reference(inner, direct, scopes, uses);
                scopes.pop();
                for (_, aggregate) in aggregates {
                    for e in aggregate.args().iter().chain(
                        aggregate
                            .order_by()
                            .iter()
                            .map(crate::modifier::order_sort_key),
                    ) {
                        classify_expression_reference(e, scopes, uses);
                    }
                }
            }
        }
    }

    /// The expression half of [`classify_reference`], with no deferred placeholders.
    fn classify_expression_reference<'a>(
        expr: &'a Expression,
        scopes: &mut Vec<Scope<'a>>,
        uses: &mut Vec<EndpointUse>,
    ) {
        crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
            match part {
                ExpressionPart::Sub(sub) => classify_expression_reference(sub, scopes, uses),
                ExpressionPart::Exists(pattern) => {
                    classify_reference(pattern, false, scopes, uses);
                }
                ExpressionPart::Call(_) => {}
            }
            false
        });
    }

    /// [`super::mentions_variable_endpoint`], as a recursion.
    fn mentions_reference(pattern: &GraphPattern) -> bool {
        if let GraphPattern::Service { name, .. } = pattern {
            return matches!(name, NamedNodePattern::Variable(_));
        }
        crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| match part {
            PatternPart::Child(child, _) => mentions_reference(child),
            PatternPart::Expression(expr) => expression_mentions_reference(expr),
        })
    }

    fn expression_mentions_reference(expr: &Expression) -> bool {
        crate::governor::soundness::visit_expression_parts(expr, &mut |part| match part {
            ExpressionPart::Sub(sub) => expression_mentions_reference(sub),
            ExpressionPart::Exists(pattern) => mentions_reference(pattern),
            ExpressionPart::Call(_) => false,
        })
    }

    impl ServedIndex {
        /// [`Self::summarize`], as a recursion over the pattern.
        fn summarize_reference(
            &mut self,
            pattern: &GraphPattern,
            ids: &crate::DetHashMap<usize, NodeId>,
        ) -> Vec<Occurrence> {
            match pattern {
                GraphPattern::Bgp { .. }
                | GraphPattern::Path { .. }
                | GraphPattern::Values { .. }
                | GraphPattern::PropertyFunction(_) => Vec::new(),
                GraphPattern::Service { name, .. } => match name {
                    NamedNodePattern::Variable(variable) => vec![Occurrence {
                        variable: variable.clone(),
                        direct: true,
                        conflict: false,
                    }],
                    NamedNodePattern::NamedNode(_) => Vec::new(),
                },
                GraphPattern::Join { left, right } => {
                    let mut summary = self.summarize_reference(left, ids);
                    let right_summary = self.summarize_reference(right, ids);
                    self.note(left, &summary, ids);
                    self.note(right, &right_summary, ids);
                    merge(&mut summary, right_summary);
                    summary
                }
                GraphPattern::Lateral { left, right } => {
                    let mut summary = self.summarize_reference(left, ids);
                    let right_summary = self.summarize_reference(right, ids);
                    merge(&mut summary, right_summary);
                    summary
                }
                GraphPattern::Union { arms } => {
                    let mut summary = Vec::new();
                    for arm in arms {
                        let arm_summary = self.summarize_reference(arm, ids);
                        merge(&mut summary, arm_summary);
                    }
                    summary
                }
                GraphPattern::LeftJoin {
                    left,
                    right,
                    expression,
                } => {
                    let mut summary = self.summarize_reference(left, ids);
                    let right_summary = self.summarize_reference(right, ids);
                    self.note(right, &right_summary, ids);
                    merge(&mut summary, indirect(right_summary));
                    if let Some(expression) = expression {
                        let expression_summary =
                            self.summarize_expression_reference(expression, ids);
                        merge(&mut summary, expression_summary);
                    }
                    summary
                }
                GraphPattern::Minus { left, right } => {
                    let mut summary = self.summarize_reference(left, ids);
                    let right_summary = self.summarize_reference(right, ids);
                    self.note(right, &right_summary, ids);
                    merge(&mut summary, indirect(right_summary));
                    summary
                }
                GraphPattern::Filter { expr, inner } => {
                    let mut summary = self.summarize_reference(inner, ids);
                    let expression_summary = self.summarize_expression_reference(expr, ids);
                    merge(&mut summary, expression_summary);
                    summary
                }
                GraphPattern::Extend {
                    inner, expression, ..
                }
                | GraphPattern::Unfold {
                    inner, expression, ..
                } => {
                    let mut summary = self.summarize_reference(inner, ids);
                    let expression_summary = self.summarize_expression_reference(expression, ids);
                    merge(&mut summary, expression_summary);
                    summary
                }
                GraphPattern::Graph { inner, .. }
                | GraphPattern::Distinct { inner }
                | GraphPattern::Reduced { inner } => self.summarize_reference(inner, ids),
                GraphPattern::OrderBy { inner, expression } => {
                    let mut summary = self.summarize_reference(inner, ids);
                    for key in expression {
                        let key_summary = self.summarize_expression_reference(
                            crate::modifier::order_sort_key(key),
                            ids,
                        );
                        merge(&mut summary, key_summary);
                    }
                    summary
                }
                GraphPattern::Project { inner, variables } => {
                    let mut summary = self.summarize_reference(inner, ids);
                    summary.retain(|o| variables.contains(&o.variable));
                    summary
                }
                GraphPattern::Slice {
                    inner,
                    start,
                    length,
                } => {
                    let summary = self.summarize_reference(inner, ids);
                    if *start == 0 && length.is_none() {
                        summary
                    } else {
                        indirect(summary)
                    }
                }
                GraphPattern::Group {
                    inner,
                    variables,
                    aggregates,
                } => {
                    let mut summary = self.summarize_reference(inner, ids);
                    for occurrence in &mut summary {
                        if !variables.contains(&occurrence.variable) {
                            occurrence.conflict |= occurrence.direct;
                            occurrence.direct = false;
                        }
                    }
                    for (_, aggregate) in aggregates {
                        for e in aggregate.args().iter().chain(
                            aggregate
                                .order_by()
                                .iter()
                                .map(crate::modifier::order_sort_key),
                        ) {
                            let expression_summary = self.summarize_expression_reference(e, ids);
                            merge(&mut summary, expression_summary);
                        }
                    }
                    summary
                }
            }
        }

        /// The expression half of [`Self::summarize_reference`].
        fn summarize_expression_reference(
            &mut self,
            expr: &Expression,
            ids: &crate::DetHashMap<usize, NodeId>,
        ) -> Vec<Occurrence> {
            let mut summary = Vec::new();
            crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
                match part {
                    ExpressionPart::Sub(sub) => {
                        let sub_summary = self.summarize_expression_reference(sub, ids);
                        merge(&mut summary, sub_summary);
                    }
                    ExpressionPart::Exists(body) => {
                        let body_summary = self.summarize_reference(body, ids);
                        let uses: Arc<[Variable]> =
                            body_summary.iter().map(|o| o.variable.clone()).collect();
                        self.exists_uses[super::scanned_id(ids, body).index()] = Some(uses);
                        merge(&mut summary, indirect(body_summary));
                    }
                    ExpressionPart::Call(_) => {}
                }
                false
            });
            summary
        }
    }

    /// [`super::lateral_endpoint_uses`], as a recursion.
    fn lateral_uses_reference(pattern: &GraphPattern, uses: &mut Vec<(Variable, bool)>) {
        match pattern {
            GraphPattern::Service { name, silent, .. } => {
                if let NamedNodePattern::Variable(variable) = name {
                    if let Some((_, all_silent)) =
                        uses.iter_mut().find(|(seen, _)| seen == variable)
                    {
                        *all_silent &= *silent;
                    } else {
                        uses.push((variable.clone(), *silent));
                    }
                }
            }
            GraphPattern::Project { inner, variables } => {
                let before = uses.len();
                lateral_uses_reference(inner, uses);
                let mut index = before;
                while index < uses.len() {
                    if variables.contains(&uses[index].0) {
                        index += 1;
                    } else {
                        uses.remove(index);
                    }
                }
            }
            _ => {
                crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
                    if let PatternPart::Child(child, _) = part {
                        lateral_uses_reference(child, uses);
                    }
                    false
                });
            }
        }
    }

    // ── The generator ───────────────────────────────────────────────────────────────

    /// A deterministic sequence of choices, drawn from one SplitMix64 counter stream.
    pub(crate) struct Choices {
        pub(crate) state: u64,
    }

    impl Choices {
        /// One of `n` alternatives.
        fn pick(&mut self, n: usize) -> usize {
            let bound = u64::try_from(n).expect("an alternative count fits a u64");
            usize::try_from(splitmix64_next(&mut self.state) % bound)
                .expect("a remainder below the count fits a usize")
        }

        fn flag(&mut self) -> bool {
            self.pick(2) == 1
        }

        /// One of the four variables the shapes draw from.
        fn variable(&mut self) -> Variable {
            Variable::new(format!("e{}", self.pick(4)))
        }

        /// A subset of those four, in name order.
        fn variables(&mut self) -> Vec<Variable> {
            (0..4)
                .filter(|_| self.flag())
                .map(|i| Variable::new(format!("e{i}")))
                .collect()
        }

        /// A `SERVICE` name: a variable three times in four, otherwise an IRI.
        fn endpoint(&mut self) -> NamedNodePattern {
            if self.pick(4) == 0 {
                NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!("{EX}endpoint")))
            } else {
                NamedNodePattern::Variable(self.variable())
            }
        }
    }

    /// A pattern that owns no other pattern.
    fn leaf(choices: &mut Choices) -> GraphPattern {
        match choices.pick(3) {
            0 => GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(choices.variable()),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!(
                        "{EX}p"
                    ))),
                    object: TermPattern::Variable(choices.variable()),
                }],
            },
            1 => GraphPattern::Values {
                variables: Vec::new(),
                bindings: Vec::new(),
            },
            _ => GraphPattern::Path {
                subject: TermPattern::Variable(choices.variable()),
                path: PropertyPathExpression::NamedNode(NamedNode::new_unchecked(format!("{EX}p"))),
                object: TermPattern::Variable(choices.variable()),
            },
        }
    }

    /// A `SERVICE` clause over an empty body.
    fn service(choices: &mut Choices) -> GraphPattern {
        GraphPattern::Service {
            name: choices.endpoint(),
            inner: Child::new(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            silent: choices.flag(),
        }
    }

    /// A pattern of at most `budget` interior nodes, every variant of the algebra reachable.
    pub(crate) fn pattern(choices: &mut Choices, budget: &mut usize) -> GraphPattern {
        if *budget == 0 {
            return if choices.flag() {
                service(choices)
            } else {
                leaf(choices)
            };
        }
        *budget -= 1;
        let child =
            |choices: &mut Choices, budget: &mut usize| Child::new(pattern(choices, budget));
        match choices.pick(17) {
            0 => leaf(choices),
            1 | 2 => service(choices),
            3 => GraphPattern::Join {
                left: child(choices, budget),
                right: child(choices, budget),
            },
            4 => GraphPattern::Lateral {
                left: child(choices, budget),
                right: child(choices, budget),
            },
            5 => GraphPattern::Minus {
                left: child(choices, budget),
                right: child(choices, budget),
            },
            6 => GraphPattern::LeftJoin {
                left: child(choices, budget),
                right: child(choices, budget),
                expression: choices.flag().then(|| expression(choices, budget)),
            },
            7 => {
                let first = pattern(choices, budget);
                let second = pattern(choices, budget);
                let rest = choices.flag().then(|| pattern(choices, budget));
                GraphPattern::Union {
                    arms: Chain::new(first, second, rest),
                }
            }
            8 => GraphPattern::Filter {
                expr: expression(choices, budget),
                inner: child(choices, budget),
            },
            9 => GraphPattern::Extend {
                inner: child(choices, budget),
                variable: choices.variable(),
                expression: expression(choices, budget),
            },
            10 => GraphPattern::Unfold {
                inner: child(choices, budget),
                expression: expression(choices, budget),
                element: choices.variable(),
                companion: choices.flag().then(|| choices.variable()),
            },
            11 => GraphPattern::Graph {
                name: NamedNodePattern::Variable(choices.variable()),
                inner: child(choices, budget),
            },
            12 => {
                let inner = child(choices, budget);
                if choices.flag() {
                    GraphPattern::Distinct { inner }
                } else {
                    GraphPattern::Reduced { inner }
                }
            }
            13 => GraphPattern::OrderBy {
                inner: child(choices, budget),
                expression: (0..choices.pick(3))
                    .map(|_| {
                        let key = expression(choices, budget);
                        if choices.flag() {
                            OrderExpression::Asc(key)
                        } else {
                            OrderExpression::Desc(key)
                        }
                    })
                    .collect(),
            },
            14 => GraphPattern::Project {
                inner: child(choices, budget),
                variables: choices.variables(),
            },
            15 => GraphPattern::Slice {
                inner: child(choices, budget),
                start: choices.pick(2),
                length: choices.flag().then(|| choices.pick(2)),
            },
            _ => {
                let inner = child(choices, budget);
                let variables = choices.variables();
                let aggregates = (0..choices.pick(3))
                    .map(|_| {
                        let aggregate = if choices.flag() {
                            AggregateExpression::new(
                                AggregateFunction::Count,
                                Vec::new(),
                                Vec::new(),
                                Vec::new(),
                                false,
                            )
                        } else {
                            AggregateExpression::new(
                                AggregateFunction::Sample,
                                vec![expression(choices, budget)],
                                Vec::new(),
                                Vec::new(),
                                false,
                            )
                        };
                        (
                            choices.variable(),
                            aggregate.expect("COUNT(*) and a one-argument SAMPLE are valid"),
                        )
                    })
                    .collect();
                GraphPattern::Group {
                    inner,
                    variables,
                    aggregates,
                }
            }
        }
    }

    /// An expression of at most `budget` interior nodes, `EXISTS` bodies included.
    fn expression(choices: &mut Choices, budget: &mut usize) -> Expression {
        if *budget == 0 {
            return Expression::Bound(choices.variable());
        }
        *budget -= 1;
        match choices.pick(6) {
            0 => Expression::Bound(choices.variable()),
            1 | 2 => Expression::Exists(Child::new(pattern(choices, budget))),
            3 => Expression::Not(Child::new(expression(choices, budget))),
            4 => {
                let first = expression(choices, budget);
                let second = expression(choices, budget);
                if choices.flag() {
                    Expression::And(Chain::new(first, second, []))
                } else {
                    Expression::Or(Chain::new(first, second, []))
                }
            }
            _ => Expression::FunctionCall(
                Function::Str,
                Args::from(vec![expression(choices, budget)]),
            ),
        }
    }

    // ── The checks ──────────────────────────────────────────────────────────────────

    /// Every walk answers what its recursive reference answers, on two hundred generated
    /// shapes: the same uses in the same order, the same summaries, the same served lists
    /// and `EXISTS` uses under the same addresses, the same `LATERAL` uses.
    #[test]
    fn every_walk_agrees_with_its_recursive_reference_on_generated_shapes() {
        let mut choices = Choices { state: 0x5EED_0358 };
        let mut with_endpoint = 0_usize;
        for shape in 0..200 {
            let mut budget = 24;
            let root = pattern(&mut choices, &mut budget);

            let mentions = super::mentions_variable_endpoint(&root);
            assert_eq!(
                mentions,
                mentions_reference(&root),
                "shape {shape}: {root:?}"
            );
            with_endpoint += usize::from(mentions);
            let tree = crate::plan::Tree::build(&root);
            assert_eq!(
                tree.shape().endpoints().is_absent(),
                !mentions,
                "shape {shape}: the scan is present exactly when a variable endpoint is"
            );

            let mut uses = Vec::new();
            super::classify(&root, true, &mut uses, None);
            let mut reference_uses = Vec::new();
            classify_reference(&root, true, &mut Vec::new(), &mut reference_uses);
            assert_eq!(uses, reference_uses, "shape {shape}: {root:?}");

            let ids = tree.shape().addresses();
            let mut index = ServedIndex::with_len(tree.shape().len());
            let summary = index.summarize(&root, ids);
            let mut reference_index = ServedIndex::with_len(tree.shape().len());
            let reference_summary = reference_index.summarize_reference(&root, ids);
            assert_eq!(summary, reference_summary, "shape {shape}: {root:?}");
            assert_eq!(
                index.served, reference_index.served,
                "shape {shape}: {root:?}"
            );
            assert_eq!(
                index.exists_uses, reference_index.exists_uses,
                "shape {shape}: {root:?}"
            );

            let mut lateral = Vec::new();
            super::lateral_endpoint_uses(&root, &mut lateral);
            let mut reference_lateral = Vec::new();
            lateral_uses_reference(&root, &mut reference_lateral);
            assert_eq!(lateral, reference_lateral, "shape {shape}: {root:?}");
        }
        assert!(
            with_endpoint > 50,
            "the generator writes a variable endpoint into most shapes: {with_endpoint}"
        );
    }

    /// Every walk answers over an operand a hundred thousand levels deep on a thread with
    /// 128 KiB of stack — a few hundred frames of any recursion — so each walks its
    /// nesting on the heap. The answers are the ones the shapes construct: a `SERVICE ?e`
    /// under a chain of `Join`s, `DISTINCT`s and `FILTER`s is served at the root's right
    /// operand, and one under a chain of `FILTER EXISTS` bodies is a use of every body and
    /// served nowhere.
    #[test]
    fn a_hundred_thousand_level_operand_is_walked_on_a_128_kib_thread() {
        const DEPTH: usize = 100_000;

        fn bottom() -> GraphPattern {
            GraphPattern::Service {
                name: NamedNodePattern::Variable(Variable::new("e")),
                inner: Child::new(GraphPattern::Bgp {
                    patterns: Vec::new(),
                }),
                silent: false,
            }
        }

        fn empty() -> Child<GraphPattern> {
            Child::new(GraphPattern::Bgp {
                patterns: Vec::new(),
            })
        }

        /// `Join(Bgp, Distinct(Filter(BOUND(?x), Join(…))))`, `DEPTH` levels, a `Join` at
        /// the root.
        fn direct_chain() -> GraphPattern {
            let mut chain = bottom();
            for level in 1..=DEPTH {
                chain = match level % 3 {
                    1 if level != DEPTH => GraphPattern::Distinct {
                        inner: Child::new(chain),
                    },
                    2 if level != DEPTH => GraphPattern::Filter {
                        expr: Expression::Bound(Variable::new("x")),
                        inner: Child::new(chain),
                    },
                    _ => GraphPattern::Join {
                        left: empty(),
                        right: Child::new(chain),
                    },
                };
            }
            chain
        }

        /// `Filter(EXISTS { Filter(EXISTS { … }, Bgp) }, Bgp)`, `DEPTH` bodies deep.
        fn exists_chain() -> GraphPattern {
            let mut chain = bottom();
            for _ in 0..DEPTH {
                chain = GraphPattern::Filter {
                    expr: Expression::Exists(Child::new(chain)),
                    inner: empty(),
                };
            }
            chain
        }

        let checked = std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let e = Variable::new("e");

                let chain = direct_chain();
                assert!(super::mentions_variable_endpoint(&chain));
                let tree = crate::plan::Tree::build(&chain);
                let super::EndpointScan::Present(index) = tree.shape().endpoints() else {
                    panic!("the chain holds a variable endpoint");
                };
                let GraphPattern::Join { right, .. } = &chain else {
                    panic!("the chain's root is a Join");
                };
                assert_eq!(
                    tree.shape()
                        .node_of(right)
                        .and_then(|right| index.served_at(right))
                        .map(AsRef::as_ref),
                    Some([e.clone()].as_slice()),
                    "the root's right operand serves ?e"
                );
                assert_eq!(index.served.iter().flatten().count(), DEPTH / 3 * 2 + 2);
                assert!(index.exists_uses.iter().all(Option::is_none));
                assert_eq!(
                    super::served_endpoint_variables(&chain, None),
                    std::slice::from_ref(&e)
                );
                let mut uses = Vec::new();
                super::lateral_endpoint_uses(&chain, &mut uses);
                assert_eq!(uses, [(e.clone(), false)]);
                drop(chain);

                let nested = exists_chain();
                assert!(super::mentions_variable_endpoint(&nested));
                let tree = crate::plan::Tree::build(&nested);
                let super::EndpointScan::Present(index) = tree.shape().endpoints() else {
                    panic!("the nested chain holds a variable endpoint");
                };
                assert_eq!(
                    index.exists_uses.iter().flatten().count(),
                    DEPTH,
                    "one entry per EXISTS body"
                );
                assert!(
                    index
                        .exists_uses
                        .iter()
                        .flatten()
                        .all(|uses| uses.as_ref() == [e.clone()]),
                    "every body uses ?e"
                );
                assert!(
                    index.served.iter().all(Option::is_none),
                    "no join operand: nothing is served"
                );
                assert!(
                    super::served_endpoint_variables(&nested, None).is_empty(),
                    "reached only through EXISTS, ?e is a conflict at the root"
                );
                let mut uses = Vec::new();
                super::lateral_endpoint_uses(&nested, &mut uses);
                assert!(uses.is_empty(), "an expression is not entered");
                drop(nested);
            })
            .expect("spawn")
            .join();
        checked.expect("the 128 KiB thread returned");
    }
}

#[cfg(test)]
mod endpoint_text_tests {
    //! The silenced-invocation endpoint text against its recursive reference, and at a
    //! hundred thousand levels on a 128 KiB thread.

    use purrdf_core::{TermBox, TermValue};

    use super::{XSD_STRING, endpoint_text};

    const DEPTH: usize = 100_000;
    const SMALL_STACK: usize = 128 * 1024;

    /// The reference: the endpoint text as a recursion, an IRI bare at the top and
    /// bracketed inside a triple term.
    fn reference_endpoint_text(value: &TermValue) -> String {
        match value {
            TermValue::Iri(iri) => iri.clone(),
            TermValue::Blank { label, .. } => format!("_:{label}"),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                ..
            } => match language {
                Some(language) => format!("\"{lexical_form}\"@{language}"),
                None if datatype == XSD_STRING => format!("\"{lexical_form}\""),
                None => format!("\"{lexical_form}\"^^<{datatype}>"),
            },
            TermValue::Triple { s, p, o } => format!(
                "<<( {} {} {} )>>",
                reference_term_text(s),
                reference_term_text(p),
                reference_term_text(o)
            ),
        }
    }

    fn reference_term_text(value: &TermValue) -> String {
        match value {
            TermValue::Iri(iri) => format!("<{iri}>"),
            other => reference_endpoint_text(other),
        }
    }

    /// A deterministic choice sequence.
    struct Choices {
        state: u64,
        budget: usize,
    }

    impl Choices {
        const fn new(seed: u64) -> Self {
            Self {
                state: seed,
                budget: 12,
            }
        }

        fn choose(&mut self, n: usize) -> usize {
            let bound = u64::try_from(n).expect("a choice count fits");
            usize::try_from(crate::test_rng::splitmix64_next(&mut self.state) % bound)
                .expect("a draw below the count fits")
        }

        fn spend(&mut self) -> bool {
            if self.budget == 0 {
                return false;
            }
            self.budget -= 1;
            true
        }
    }

    fn leaf(choices: &mut Choices) -> TermValue {
        match choices.choose(5) {
            0 => TermValue::iri("http://example.org/a"),
            1 => TermValue::Blank {
                label: "b1".to_owned(),
                scope: purrdf_core::BlankScope::DEFAULT,
            },
            2 => TermValue::Literal {
                lexical_form: "plain".to_owned(),
                datatype: XSD_STRING.to_owned(),
                language: None,
                direction: None,
            },
            3 => TermValue::Literal {
                lexical_form: "tagged".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some("en".to_owned()),
                direction: None,
            },
            _ => TermValue::Literal {
                lexical_form: "7".to_owned(),
                datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                language: None,
                direction: None,
            },
        }
    }

    fn term(choices: &mut Choices) -> TermValue {
        if !choices.spend() || choices.choose(3) == 0 {
            return leaf(choices);
        }
        TermValue::Triple {
            s: TermBox::new(term(choices)),
            p: TermBox::new(term(choices)),
            o: TermBox::new(term(choices)),
        }
    }

    /// The work-list writer spells every generated term exactly as the reference does,
    /// IRIs bare at the top and bracketed inside, nested triple terms included.
    #[test]
    fn endpoint_text_agrees_with_its_recursive_reference_on_generated_terms() {
        let mut nested = 0;
        for seed in 0..400_u64 {
            let mut choices = Choices::new(seed);
            let value = term(&mut choices);
            let mut depth = 0;
            let mut pending = vec![(&value, 1_usize)];
            while let Some((term, level)) = pending.pop() {
                if let TermValue::Triple { s, p, o } = term {
                    depth = depth.max(level);
                    pending.extend([(&**s, level + 1), (&**p, level + 1), (&**o, level + 1)]);
                }
            }
            nested += usize::from(depth >= 2);
            assert_eq!(
                endpoint_text(&value),
                reference_endpoint_text(&value),
                "seed {seed}: {value:?}"
            );
        }
        assert!(
            nested > 0,
            "some generated term nests a triple term in a triple term"
        );
        assert_eq!(
            endpoint_text(&TermValue::iri("http://example.org/e")),
            "http://example.org/e",
            "the endpoint's own IRI is bare"
        );
    }

    /// A triple term a hundred thousand levels deep is spelled on a 128 KiB thread: every
    /// level opens with its subject and predicate and closes after its object, the
    /// innermost object bracketed.
    #[test]
    fn a_hundred_thousand_level_term_is_spelled_on_a_128_kib_thread() {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(|| {
                let mut value = TermValue::iri("http://example.org/o");
                for _ in 0..DEPTH {
                    value = TermValue::Triple {
                        s: TermBox::new(TermValue::iri("http://example.org/s")),
                        p: TermBox::new(TermValue::iri("http://example.org/p")),
                        o: TermBox::new(value),
                    };
                }
                let text = endpoint_text(&value);
                let level = "<<( <http://example.org/s> <http://example.org/p> ";
                let close = " )>>";
                let innermost = "<http://example.org/o>";
                // Every level's opening, then the innermost object, then every level's
                // closing: the innermost object is followed by all `DEPTH` closings.
                let expected = format!("{}{innermost}{}", level.repeat(DEPTH), close.repeat(DEPTH));
                assert_eq!(
                    text.len(),
                    DEPTH * (level.len() + close.len()) + innermost.len()
                );
                assert!(text == expected, "the spelling of every level, in order");
                drop(value);
            })
            .expect("spawn")
            .join()
            .expect("the 128 KiB thread returned");
    }
}
