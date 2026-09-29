// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A nested `EXISTS` body is substituted when it is evaluated, not when the pattern
//! around it is.
//!
//! # The cost this removes
//!
//! A correlated `EXISTS` is answered, per outer row μ, by substituting μ into its body and
//! evaluating the copy ([`crate::binop::eval_correlated`]). SPARQL's `substitute(P, μ)`
//! reaches every variable of `P`, including those of an `EXISTS` nested inside `P`, and the
//! walk used to copy that nested body into the per-row copy as well. For `d` levels of
//! `FILTER EXISTS`, each level's body holding the next, one outer row therefore:
//!
//! * copied the whole remaining chain at every level, and each level's copy re-wrapped
//!   every leaf below it in one more one-row `VALUES` join (the leaves already carried one
//!   per enclosing level): level `k`'s copy held `O((d - k) · k)` nodes, `O(d³)` a row, all
//!   of them alive at once while the innermost level ran;
//! * at every `FILTER` of every copy, walked the whole chain below it for its variables
//!   (the widened `crate::expr::expr_vars`): `O((d - k)²)` a level, `O(d³)` a row;
//! * re-prepared at every level the body it had just copied — normal form, a second copy,
//!   the structural analysis — because a copy's address is no cache key: `O(d²)` a row.
//!
//! # What happens instead
//!
//! The substitution walk leaves each nested `EXISTS` it meets as a **placeholder**: a
//! one-node pattern standing in the copy where the substituted body would have been, and
//! an entry in the window's [`DeferredMap`] naming the body's [`ExistsSite`] — prepared
//! once per evaluation, from the written body — and the [`SubstitutionEnv`] the walk would
//! have applied to it. When the `EXISTS` is evaluated for the current row ν
//! (`crate::expr::exists`), the body is substituted with the environment and ν together,
//! once: that level's own nodes are copied, and the level below it is again a placeholder.
//! One outer row now costs the size of each level, once.
//!
//! # Why the answer is the same
//!
//! Substituting `μ₁` and then `μ₂` into a pattern, and substituting their union once, build
//! the same pattern up to redundant one-row `VALUES` joins, as long as the two agree on
//! every variable both bind: every arm of the walk reads a variable from the first row
//! that binds it, a `Bgp`/`Path` leaf is joined with one `VALUES` row per binding row (and
//! equal rows join as one), and a `Project` narrows each row the same way. Where two
//! layers DO disagree — reachable only through a `BIND`/`VALUES` that rebinds an outer
//! variable, the collision SEP-0007 leaves undefined and the parser refuses, but which
//! hand-built algebra can still reach — the environment keeps every layer, and the body is
//! substituted one layer at a time, exactly as before ([`SubstitutionEnv::layers`]).
//!
//! The body is normalized ([`crate::enf`]) before substitution rather than after. The
//! normal form only erases spine wrappers that cannot change whether the body is empty
//! and cannot fail; substitution never makes a subtree able to fail, so every erasure the
//! written body admits, the substituted one admits too.
//!
//! # Where a placeholder is visible
//!
//! Anything that inspects the copy while it is evaluated, and would have descended into
//! the nested body, reads the site instead: the fork-safety gate
//! ([`crate::eval::EvalCtx::may_fork_row_loop`], the `UNION` arm gate) reads
//! [`ExistsSite::parallel_unsafe`], the variable-endpoint `SERVICE` analysis reads
//! [`ExistsSite::service_uses`], and the substitution walk itself reads
//! [`ExistsSite::vars`] where it used to walk the body for its variables. A `SERVICE` body
//! is forwarded as text, so nothing below a `SERVICE` is ever left as a placeholder.
//!
//! # A nested `LATERAL` right operand is deferred the same way
//!
//! A `LATERAL` evaluates its right operand once per left row, substituted
//! ([`crate::binop::eval_lateral`]), and a `LATERAL` nested in that right operand did the
//! same inside every copy: `d` nested levels cost what `d` nested `EXISTS` cost above —
//! each level copied the whole remaining chain, one more one-row `VALUES` join around
//! every leaf below it, and every level's copy stayed alive while the innermost ran. Time
//! and memory grew with the cube of `d`.
//!
//! So the substitution walk leaves a nested `LATERAL`'s right operand as a **lateral
//! placeholder** too: a [`LateralSite`] names it, and the window's [`DeferredMap`] carries
//! the [`SubstitutionEnv`] the walk would have applied to it. When the copied `LATERAL`
//! evaluates its right side for a left row ν, the site's body is substituted with the
//! environment and ν as one row, once — the same merge, under the same agreement
//! condition and with the same layer-by-layer fallback, as a deferred `EXISTS` body.
//!
//! A site owns its body: the right operand as written, with each `LATERAL` right operand
//! nested in it cut out into a site of its own (the site's `children`). A whole
//! chain is cut from ONE copy of the outermost deferred operand, prepared once per
//! evaluation and kept where the substitution source's sites are kept, so each of its
//! nodes is copied once per evaluation and each level's own nodes once per left row.
//!
//! A lateral placeholder stands in a pattern position, where more of the evaluator can
//! see it than an `EXISTS` placeholder's expression position: it is a `SELECT` of the
//! operand's own columns over the `EXISTS` placeholder, so the schema read off the copy
//! without evaluating it is the one the operand has, and the fork-safety walks judge it
//! by [`LateralSite::parallel_unsafe`]. An operand the variable-endpoint `SERVICE`
//! analysis reads as written — one holding a `SERVICE ?v` — and a property-function
//! call, which `crate::binop::eval_lateral` drives itself, are never deferred.

use std::sync::{Arc, OnceLock};

use purrdf_core::DatasetView;
use purrdf_sparql_algebra::{Child, Expression, GraphPattern, NamedNodePattern, Variable};

use crate::DetHashSet;
use crate::eval::{EvalCtx, PreparedExists};
use crate::expr::{SubstitutionRow, SubstitutionSource, SubstitutionSourceMap};
use crate::governor::soundness::{ExpressionPart, PatternPart};

/// One nested `EXISTS` body, prepared once per evaluation from the body as written.
pub(crate) struct ExistsSite {
    /// The body's preparation: its normal form, the first-witness form and the analysis.
    pub(crate) prepared: Arc<PreparedExists>,
    /// Every variable the written body mentions anywhere
    /// ([`crate::expr::pattern_all_vars`]): the variables a substitution can reach in it,
    /// and so the only ones an environment needs to carry for it.
    pub(crate) vars: DetHashSet<Variable>,
    /// Whether the body reaches a builtin or callee a forked worker must not run
    /// ([`crate::parallel::is_parallel_safe_pattern`]).
    pub(crate) parallel_unsafe: bool,
    /// Every variable-endpoint `SERVICE ?v` in the body that no `SELECT` inside the body
    /// hides — what the endpoint analysis would have recorded for it. Empty when the
    /// query has no variable-endpoint `SERVICE` at all.
    pub(crate) service_uses: Vec<Variable>,
    /// The preparation's nodes mapped one hop to the plan nodes the ledger indexes; empty
    /// without a ledger.
    pub(crate) plan_map: Arc<SubstitutionSourceMap>,
}

/// The substitution a nested `EXISTS` body is owed, carried rather than applied.
#[derive(Clone, Default)]
pub(crate) struct SubstitutionEnv {
    /// Every layer applied so far, newest first, each restricted to the site's variables;
    /// `None` when no layer binds any of them.
    layers: Option<Arc<EnvLayer>>,
    /// The layers merged into one row, or `None` when two of them bind a shared variable
    /// differently. Meaningful only when `layers` is `Some`.
    merged: Option<Arc<SubstitutionRow>>,
}

/// One substitution layer.
struct EnvLayer {
    row: SubstitutionRow,
    parent: Option<Arc<Self>>,
}

/// What a [`SubstitutionEnv`] amounts to.
pub(crate) enum EnvState<'a> {
    /// Nothing to substitute: the written body is the body.
    Empty,
    /// One row substitutes every layer.
    Merged(&'a SubstitutionRow),
    /// The layers disagree; they must be applied one at a time.
    Layered,
}

impl SubstitutionEnv {
    /// This environment with `row` applied after it, restricted to `vars`.
    pub(crate) fn then(&self, row: &SubstitutionRow, vars: &DetHashSet<Variable>) -> Self {
        let pruned = restrict(row, vars);
        if pruned.term.is_empty() && pruned.expr.is_empty() {
            return self.clone();
        }
        let merged = match (&self.layers, &self.merged) {
            (None, _) => Some(Arc::new(pruned.clone())),
            (Some(_), Some(base)) => merge(base, &pruned).map(Arc::new),
            (Some(_), None) => None,
        };
        Self {
            layers: Some(Arc::new(EnvLayer {
                row: pruned,
                parent: self.layers.clone(),
            })),
            merged,
        }
    }

    /// What this environment amounts to.
    pub(crate) fn state(&self) -> EnvState<'_> {
        match (&self.layers, &self.merged) {
            (None, _) => EnvState::Empty,
            (Some(_), Some(row)) => EnvState::Merged(row),
            (Some(_), None) => EnvState::Layered,
        }
    }

    /// Every layer, oldest first.
    pub(crate) fn layers(&self) -> Vec<&SubstitutionRow> {
        let mut out = Vec::new();
        let mut next = self.layers.as_deref();
        while let Some(layer) = next {
            out.push(&layer.row);
            next = layer.parent.as_deref();
        }
        out.reverse();
        out
    }

    /// Whether substituting this environment turns a `SERVICE ?variable` into a clause
    /// that no longer needs an enclosing operator's endpoint list: any binding does. An
    /// IRI binding becomes the clause's endpoint; any other term is joined beside the
    /// clause as a one-row `VALUES`, which lists it as the clause's only endpoint (see
    /// `crate::expr::substitute_pattern`). Layer by layer, the first layer that binds it
    /// decides, so "any layer" is the answer.
    pub(crate) fn resolves_endpoint(&self, variable: &Variable) -> bool {
        let mut next = self.layers.as_deref();
        while let Some(layer) = next {
            let row = &layer.row;
            if row.expr.iter().any(|(v, _)| v == variable)
                || row.term.iter().any(|(v, _)| v == variable)
            {
                return true;
            }
            next = layer.parent.as_deref();
        }
        false
    }
}

/// `row` restricted to `vars`, each side in its own order.
fn restrict(row: &SubstitutionRow, vars: &DetHashSet<Variable>) -> SubstitutionRow {
    SubstitutionRow {
        expr: row
            .expr
            .iter()
            .filter(|(v, _)| vars.contains(v))
            .cloned()
            .collect(),
        term: row
            .term
            .iter()
            .filter(|(v, _)| vars.contains(v))
            .cloned()
            .collect(),
    }
}

/// `base` followed by `next`, as one row: `None` when they bind a shared variable to
/// different terms. Where they agree, the term (and so the expression constant, which is
/// a function of the term) is the same, so the merged row keeps `base`'s entry.
fn merge(base: &SubstitutionRow, next: &SubstitutionRow) -> Option<SubstitutionRow> {
    let mut out = base.clone();
    for (v, t) in &next.term {
        match base.term.iter().find(|(bv, _)| bv == v) {
            Some((_, bt)) if bt != t => return None,
            Some(_) => {}
            None => out.term.push((v.clone(), t.clone())),
        }
    }
    for (v, e) in &next.expr {
        if !base.expr.iter().any(|(bv, _)| bv == v) {
            out.expr.push((v.clone(), e.clone()));
        }
    }
    Some(out)
}

/// The environment `env` followed by the current row `row`, restricted to `vars`, as one
/// row — or `None` when some layer disagrees with another.
pub(crate) fn with_row(
    env: &SubstitutionRow,
    row: &SubstitutionRow,
    vars: &DetHashSet<Variable>,
) -> Option<SubstitutionRow> {
    merge(env, &restrict(row, vars))
}

/// A placeholder's site and the substitution it is owed.
#[derive(Clone)]
pub(crate) struct DeferredExists {
    /// The body.
    pub(crate) site: Arc<ExistsSite>,
    /// What to substitute into it.
    pub(crate) env: SubstitutionEnv,
}

/// A lateral placeholder's site and the substitution it is owed.
#[derive(Clone)]
pub(crate) struct DeferredLateral {
    /// The right operand.
    pub(crate) site: Arc<LateralSite>,
    /// What to substitute into it, before the left row it is evaluated for.
    pub(crate) env: SubstitutionEnv,
}

/// What one placeholder of a substituted copy stands for.
#[derive(Clone)]
pub(crate) enum Deferred {
    /// A nested `EXISTS` body, in an expression position.
    Exists(DeferredExists),
    /// A nested `LATERAL`'s right operand, in that operand's position.
    Lateral(DeferredLateral),
}

impl Deferred {
    /// The `EXISTS` body this placeholder stands for, if it stands for one.
    pub(crate) const fn exists(&self) -> Option<&DeferredExists> {
        match self {
            Self::Exists(slot) => Some(slot),
            Self::Lateral(_) => None,
        }
    }

    /// The `LATERAL` right operand this placeholder stands for, if it stands for one.
    pub(crate) const fn lateral(&self) -> Option<&DeferredLateral> {
        match self {
            Self::Lateral(slot) => Some(slot),
            Self::Exists(_) => None,
        }
    }

    /// Whether the body or operand reaches a builtin or callee a forked worker must not
    /// run.
    pub(crate) fn parallel_unsafe(&self) -> bool {
        match self {
            Self::Exists(slot) => slot.site.parallel_unsafe,
            Self::Lateral(slot) => slot.site.parallel_unsafe,
        }
    }
}

/// A substituted copy's placeholders, by address.
pub(crate) type DeferredMap = crate::DetHashMap<usize, Deferred>;

/// The sites of the `EXISTS` bodies a substitution source holds, by the body's address —
/// what the map dereferences to — and of its deferred `LATERAL` right operands, by the
/// operand's address.
#[derive(Default)]
pub(crate) struct NestedSites {
    exists: crate::DetHashMap<usize, Arc<ExistsSite>>,
    pub(crate) lateral: crate::DetHashMap<usize, Arc<LateralSite>>,
}

impl std::ops::Deref for NestedSites {
    type Target = crate::DetHashMap<usize, Arc<ExistsSite>>;
    fn deref(&self) -> &Self::Target {
        &self.exists
    }
}

impl std::ops::DerefMut for NestedSites {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.exists
    }
}

/// A nested `LATERAL`'s right operand, cut out of the pattern that holds it and prepared
/// once per evaluation — see this module's doc.
pub(crate) struct LateralSite {
    /// The operand as written, each deferrable `LATERAL` right operand nested in it
    /// replaced by a lateral placeholder whose site is in `children`. Boxed, so the
    /// addresses [`Self::plan_map`] and `children` are keyed by never move.
    pub(crate) body: Box<GraphPattern>,
    /// The columns the uncut operand exposes, in its schema order: the columns its
    /// placeholder exposes.
    pub(crate) schema: Vec<Variable>,
    /// Every variable the uncut operand mentions anywhere: the variables a substitution
    /// can reach in it, and so the only ones an environment needs to carry for it.
    pub(crate) vars: DetHashSet<Variable>,
    /// Whether the uncut operand reaches a builtin or callee a forked worker must not run.
    pub(crate) parallel_unsafe: bool,
    /// Every node of the whole cut chain mapped one hop to the plan node it copies; empty
    /// without a ledger. One map, shared by every site cut from the same copy.
    pub(crate) plan_map: Arc<SubstitutionSourceMap>,
    /// The sites of the operands cut out of [`Self::body`], by their placeholder's
    /// address.
    children: crate::DetHashMap<usize, Arc<Self>>,
    /// The sites of the body's own nested `EXISTS` bodies and of `children`, prepared on
    /// the body's first substitution.
    sites: OnceLock<Arc<NestedSites>>,
}

/// The sites of the `EXISTS` bodies of the plan nodes a `LATERAL` substitutes, by the
/// node's address, for one evaluation.
///
/// A preparation keeps the sites of its own bodies itself
/// ([`PreparedExists::nested_sites`]); a plan node has nowhere to keep them, so they are
/// kept here. Shared by every worker the evaluation forks (one `Arc`), so a worker does
/// not drop what it prepared; read-mostly, so readers never wait on each other. The plan
/// outlives the evaluation, so an address key cannot be reused while the entry exists.
#[derive(Default)]
pub(crate) struct PlanSites {
    by_root: std::sync::RwLock<crate::DetHashMap<usize, Arc<NestedSites>>>,
}

/// Where the sites of a substitution source's nested `EXISTS` bodies are kept.
#[derive(Clone, Copy)]
pub(crate) enum SiteSlot<'a> {
    /// The source belongs to this preparation, which owns the tree the bodies are in and
    /// keeps their sites for as long as it lives.
    Prepared(&'a OnceLock<Arc<NestedSites>>),
    /// The source is a plan node: its sites are kept on the evaluation's [`PlanSites`].
    Plan,
    /// The source is part of a per-row copy: its sites are prepared for this use alone.
    Transient,
    /// The source is this site's body, which keeps the sites for as long as it lives.
    Lateral(&'a LateralSite),
}

/// Where a correlated substitution reads its pattern from.
#[derive(Clone, Copy)]
pub(crate) struct CorrelatedSource<'a> {
    /// Where the sites of the pattern's nested `EXISTS` bodies are kept.
    pub(crate) sites: SiteSlot<'a>,
    /// The pattern's nodes mapped one hop to plan nodes, when the pattern is not itself
    /// part of the plan (an `EXISTS` preparation).
    pub(crate) plan_map: Option<&'a SubstitutionSourceMap>,
}

/// The sites of a source's bodies: borrowed from where they are kept, or prepared for
/// one use.
pub(crate) enum SitesRef<'a> {
    Kept(&'a NestedSites),
    Owned(Arc<NestedSites>),
}

impl std::ops::Deref for SitesRef<'_> {
    type Target = NestedSites;
    fn deref(&self) -> &NestedSites {
        match self {
            Self::Kept(sites) => sites,
            Self::Owned(sites) => sites,
        }
    }
}

/// The name of the variable a placeholder carries: no SPARQL variable can be spelled with
/// a space, so it names no variable of any query.
const PLACEHOLDER_VARIABLE: &str = "purrdf deferred exists";

/// The placeholder standing in a copy for a nested `EXISTS` body: a `VALUES` block with no
/// rows over a variable no query can name. Its address keys the window's [`DeferredMap`];
/// its shape lets [`is_placeholder`] tell a placeholder that lost its entry (which would
/// be a bug) from a body, so it can never be evaluated as one.
#[allow(
    clippy::unnecessary_box_returns,
    reason = "the box's heap address is the placeholder's identity: it keys the DeferredMap \
              and must be fixed before the placeholder moves into the substituted copy \
              (the lint fires only where GraphPattern is small, i.e. on wasm32)"
)]
pub(crate) fn placeholder() -> Box<GraphPattern> {
    Box::new(GraphPattern::Values {
        variables: vec![Variable::new(PLACEHOLDER_VARIABLE)],
        bindings: Vec::new(),
    })
}

/// Whether `pattern` has the shape [`placeholder`] builds.
pub(crate) fn is_placeholder(pattern: &GraphPattern) -> bool {
    matches!(
        pattern,
        GraphPattern::Values { variables, bindings }
            if bindings.is_empty()
                && variables.len() == 1
                && variables[0].as_str() == PLACEHOLDER_VARIABLE
    )
}

/// The placeholder standing in a copy for a nested `LATERAL`'s right operand, whose
/// columns are `schema`: a `SELECT` of those columns over a [`placeholder`], so the
/// columns read off the copy without evaluating it are the operand's. Its address keys
/// the window's [`DeferredMap`]; its shape lets [`is_lateral_placeholder`] tell one that
/// lost its entry from an operand, so it is never evaluated as one.
#[allow(
    clippy::unnecessary_box_returns,
    reason = "the box's heap address is the placeholder's identity: it keys the DeferredMap \
              and must be fixed before the placeholder moves into the substituted copy \
              (the lint fires only where GraphPattern is small, i.e. on wasm32)"
)]
pub(crate) fn lateral_placeholder(schema: &[Variable]) -> Box<GraphPattern> {
    Box::new(GraphPattern::Project {
        inner: Child::from(placeholder()),
        variables: schema.to_vec(),
    })
}

/// Whether `pattern` has the shape [`lateral_placeholder`] builds.
pub(crate) fn is_lateral_placeholder(pattern: &GraphPattern) -> bool {
    matches!(pattern, GraphPattern::Project { inner, .. } if is_placeholder(inner))
}

/// The sites of every `EXISTS` body and every deferrable `LATERAL` right operand
/// `pattern` holds outside a `SERVICE` body, outside another `EXISTS` body and outside
/// another deferred operand, other than the placeholders of the window being substituted
/// from — the bodies and operands a substitution of `pattern` will leave as placeholders.
/// Prepared on the first substitution of `pattern` and kept where `source` says, so a
/// later outer row reads them without taking a lock (a preparation's or a site's own) or
/// with a shared read lock (a plan node's).
pub(crate) fn nested_sites<'a, D: DatasetView + Sync>(
    pattern: &GraphPattern,
    source: CorrelatedSource<'a>,
    ctx: &mut EvalCtx<'_, D>,
) -> SitesRef<'a> {
    let root = std::ptr::from_ref(pattern) as usize;
    match source.sites {
        SiteSlot::Prepared(slot) => {
            if let Some(kept) = slot.get() {
                return SitesRef::Kept(kept);
            }
            let sites = Arc::new(prepare_sites(pattern, source, ctx));
            // Another worker may have kept its own first; both are the same sites.
            let kept = slot.get_or_init(|| sites);
            SitesRef::Kept(kept)
        }
        SiteSlot::Plan => {
            let plan_sites = Arc::clone(ctx.plan_exists_sites.get_or_insert_with(Arc::default));
            let kept = plan_sites
                .by_root
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&root)
                .cloned();
            if let Some(kept) = kept {
                return SitesRef::Owned(kept);
            }
            let sites = Arc::new(prepare_sites(pattern, source, ctx));
            let kept = Arc::clone(
                plan_sites
                    .by_root
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .entry(root)
                    .or_insert(sites),
            );
            SitesRef::Owned(kept)
        }
        SiteSlot::Transient => SitesRef::Owned(Arc::new(prepare_sites(pattern, source, ctx))),
        SiteSlot::Lateral(site) => {
            let kept = site
                .sites
                .get_or_init(|| Arc::new(prepare_sites(pattern, source, ctx)));
            SitesRef::Kept(kept)
        }
    }
}

/// Prepare the site of every body and operand [`nested_sites`] names.
fn prepare_sites<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    source: CorrelatedSource<'_>,
    ctx: &EvalCtx<'_, D>,
) -> NestedSites {
    let mut sites = NestedSites::default();
    let deferrable = Deferrable::of(ctx);
    let (bodies, operands) = nested_bodies(pattern, ctx.deferred_exists.as_deref(), deferrable);
    for body in bodies {
        let site = build_site(body, source, ctx);
        sites.insert(std::ptr::from_ref(body) as usize, Arc::new(site));
    }
    for operand in operands {
        let site = build_lateral_site(operand, source, deferrable, ctx);
        sites
            .lateral
            .insert(std::ptr::from_ref(operand) as usize, site);
    }
    if let SiteSlot::Lateral(site) = source.sites {
        sites.lateral.extend(
            site.children
                .iter()
                .map(|(&address, child)| (address, Arc::clone(child))),
        );
    }
    sites
}

/// Which `LATERAL` right operands the substitution walk leaves as placeholders.
#[derive(Clone, Copy)]
struct Deferrable {
    /// The query holds no variable-endpoint `SERVICE`, so no operand needs the endpoint
    /// analysis to read it as written.
    no_variable_endpoint: bool,
}

impl Deferrable {
    fn of<D: DatasetView + Sync>(ctx: &EvalCtx<'_, D>) -> Self {
        Self {
            no_variable_endpoint: ctx.endpoint_scan().is_absent(),
        }
    }

    /// Whether `operand`, the right operand of a `LATERAL`, is deferred: not a
    /// property-function call (`crate::binop::eval_lateral` drives one per left row
    /// itself), not already a placeholder, and holding no variable-endpoint `SERVICE` the
    /// endpoint analysis of an enclosing operator must read as written.
    fn operand(self, operand: &GraphPattern) -> bool {
        !matches!(operand, GraphPattern::PropertyFunction(_))
            && !is_lateral_placeholder(operand)
            && (self.no_variable_endpoint
                || !crate::service_endpoints::mentions_variable_endpoint(operand))
    }
}

/// The `EXISTS` bodies and the `LATERAL` right operands [`nested_sites`] prepares, in no
/// particular order: neither is entered, and neither is a `SERVICE` body or a
/// placeholder of the window being substituted from. A worklist rather than a recursion:
/// a flat operator spine can be far deeper than the nesting of `EXISTS`.
fn nested_bodies<'p>(
    pattern: &'p GraphPattern,
    placeholders: Option<&DeferredMap>,
    deferrable: Deferrable,
) -> (Vec<&'p GraphPattern>, Vec<&'p GraphPattern>) {
    enum Node<'p> {
        Pattern(&'p GraphPattern),
        Expression(&'p Expression),
    }
    let mut bodies = Vec::new();
    let mut operands = Vec::new();
    let mut pending = vec![Node::Pattern(pattern)];
    while let Some(node) = pending.pop() {
        match node {
            // A `SERVICE` body is substituted in full: it is forwarded as text.
            Node::Pattern(GraphPattern::Service { .. }) => {}
            Node::Pattern(GraphPattern::Lateral { left, right }) => {
                if deferrable.operand(right) {
                    operands.push(&**right);
                } else if !is_lateral_placeholder(right) {
                    pending.push(Node::Pattern(right));
                }
                pending.push(Node::Pattern(left));
            }
            Node::Pattern(pattern) => {
                crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
                    pending.push(match part {
                        PatternPart::Child(child, _) => Node::Pattern(child),
                        PatternPart::Expression(expr) => Node::Expression(expr),
                    });
                    false
                });
            }
            Node::Expression(expr) => {
                crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
                    match part {
                        ExpressionPart::Sub(sub) => pending.push(Node::Expression(sub)),
                        ExpressionPart::Exists(body) => {
                            let address = std::ptr::from_ref(body) as usize;
                            if !placeholders.is_some_and(|map| map.contains_key(&address)) {
                                bodies.push(body);
                            }
                        }
                        ExpressionPart::Call(_) => {}
                    }
                    false
                });
            }
        }
    }
    (bodies, operands)
}

/// Call `visit` with the right operand of every `LATERAL` in `body` outside a `SERVICE`
/// body and outside an expression, entering the operand when `visit` answers `true`. A
/// worklist, so a body of any height costs heap and never stack.
fn visit_lateral_operands_mut(
    body: &mut GraphPattern,
    visit: &mut impl FnMut(&mut Child<GraphPattern>) -> bool,
) {
    let mut pending: Vec<&mut GraphPattern> = vec![body];
    while let Some(node) = pending.pop() {
        match node {
            GraphPattern::Lateral { left, right } => {
                if visit(right) {
                    pending.push(&mut **right);
                }
                pending.push(&mut **left);
            }
            node => crate::blank_scope::for_each_child_mut(node, &mut |child| {
                if let crate::blank_scope::ChildMut::Pattern(child) = child {
                    pending.push(child);
                }
            }),
        }
    }
}

/// Prepare the site of `operand`, the right operand of a `LATERAL`, read from `source`:
/// one copy of it, cut into a site per nested deferrable operand — see this module's doc.
///
/// The copy is cut top-down, over a queue of the pieces still to cut, and the sites are
/// assembled bottom-up, each once its children are: every step reads only its own
/// piece, so the whole chain costs its size once, and no step recurses.
fn build_lateral_site<D: DatasetView + Sync>(
    operand: &GraphPattern,
    source: CorrelatedSource<'_>,
    deferrable: Deferrable,
    ctx: &EvalCtx<'_, D>,
) -> Arc<LateralSite> {
    /// One cut piece: its body, and the piece and the placeholder it was cut out of.
    struct Piece {
        body: Box<GraphPattern>,
        parent: Option<(usize, usize)>,
    }

    let copy = Box::new(operand.clone());
    #[cfg(test)]
    crate::op_count::count_copied(purrdf_sparql_algebra::NodeRef::Pattern(operand));
    let plan_map = if ctx.ledger.is_some() {
        let mut map = SubstitutionSourceMap::default();
        crate::enf::map_clone(operand, &copy, &mut map);
        Arc::new(to_plan(&map, source.plan_map))
    } else {
        Arc::default()
    };

    // Cut: each deferrable operand leaves its piece for a placeholder, and is cut in turn.
    // A piece is numbered after the piece it was cut from, so children follow parents.
    let mut pieces: Vec<Piece> = Vec::new();
    let mut queue = vec![Piece {
        body: copy,
        parent: None,
    }];
    while let Some(Piece { mut body, parent }) = queue.pop() {
        let index = pieces.len();
        visit_lateral_operands_mut(&mut body, &mut |right| {
            if !deferrable.operand(right) {
                return true;
            }
            let cut = std::mem::replace(right, Child::from(lateral_placeholder(&[])));
            queue.push(Piece {
                body: cut.into_box(),
                parent: Some((index, std::ptr::from_ref::<GraphPattern>(&**right) as usize)),
            });
            false
        });
        pieces.push(Piece { body, parent });
    }

    // Assemble, deepest piece first: a piece's children are built before it, so its
    // placeholders take their columns and its site reads theirs.
    let mut children_of: Vec<Vec<(usize, usize)>> = vec![Vec::new(); pieces.len()];
    for (index, piece) in pieces.iter().enumerate() {
        if let Some((parent, placeholder)) = piece.parent {
            children_of[parent].push((placeholder, index));
        }
    }
    let mut built: Vec<Option<Arc<LateralSite>>> = vec![None; pieces.len()];
    while let Some(Piece { mut body, .. }) = pieces.pop() {
        let index = pieces.len();
        let children: crate::DetHashMap<usize, Arc<LateralSite>> = children_of[index]
            .iter()
            .map(|&(placeholder, child)| {
                let site = built[child]
                    .take()
                    .expect("a piece is assembled before the piece it was cut from");
                (placeholder, site)
            })
            .collect();
        if !children.is_empty() {
            visit_lateral_operands_mut(&mut body, &mut |right| {
                let address = std::ptr::from_ref::<GraphPattern>(&**right) as usize;
                match (children.get(&address), &mut **right) {
                    (Some(child), GraphPattern::Project { variables, .. }) => {
                        variables.clone_from(&child.schema);
                        false
                    }
                    _ => true,
                }
            });
        }
        let schema = crate::eval::syntactic_schema(&body).vars().to_vec();
        let mut vars = DetHashSet::default();
        crate::expr::pattern_all_vars(&body, &mut vars);
        for child in children.values() {
            vars.extend(child.vars.iter().cloned());
        }
        let parallel_unsafe = !crate::parallel::is_parallel_safe_pattern_with(
            &body,
            ctx.safety_registries(),
            &|pattern| {
                children
                    .get(&(std::ptr::from_ref(pattern) as usize))
                    .map(|child| child.parallel_unsafe)
            },
        );
        built[index] = Some(Arc::new(LateralSite {
            body,
            schema,
            vars,
            parallel_unsafe,
            plan_map: Arc::clone(&plan_map),
            children,
            sites: OnceLock::new(),
        }));
    }
    built[0]
        .take()
        .expect("the operand itself is the first piece")
}

/// Prepare the site of `body`, read from `source`.
fn build_site<D: DatasetView + Sync>(
    body: &GraphPattern,
    source: CorrelatedSource<'_>,
    ctx: &EvalCtx<'_, D>,
) -> ExistsSite {
    let prepared = Arc::new(PreparedExists::build(body));
    let mut vars = DetHashSet::default();
    crate::expr::pattern_all_vars(body, &mut vars);
    let parallel_unsafe = !crate::parallel::is_parallel_safe_pattern(body, ctx.safety_registries());
    let service_uses = match ctx.endpoint_scan() {
        crate::service_endpoints::EndpointScan::Absent => Vec::new(),
        crate::service_endpoints::EndpointScan::Present(index) => {
            match ctx.plan_node(body).and_then(|body| index.exists_uses(body)) {
                Some(uses) => uses.to_vec(),
                None => endpoint_uses(body),
            }
        }
    };
    let plan_map = match (&ctx.ledger, prepared.as_ref()) {
        (Some(_), PreparedExists::Pattern { ledger_source, .. }) => {
            Arc::new(to_plan(ledger_source, source.plan_map))
        }
        _ => Arc::new(SubstitutionSourceMap::default()),
    };
    ExistsSite {
        prepared,
        vars,
        parallel_unsafe,
        service_uses,
        plan_map,
    }
}

/// `map`, each entry's source carried one hop further through `outer` when `outer` maps it.
fn to_plan(
    map: &SubstitutionSourceMap,
    outer: Option<&SubstitutionSourceMap>,
) -> SubstitutionSourceMap {
    let Some(outer) = outer else {
        return map.clone();
    };
    map.iter()
        .map(|(&address, entry)| {
            let source = outer.get(&entry.source).map_or(entry.source, |e| e.source);
            (
                address,
                SubstitutionSource {
                    source,
                    counts_rows: entry.counts_rows,
                },
            )
        })
        .collect()
}

/// Every variable-endpoint `SERVICE ?v` in `body` that no `SELECT` inside `body` hides,
/// with its `SILENT` flag. A `SERVICE` body is not entered: it is forwarded as text, and
/// the analysis it stands in for does not enter it either.
fn endpoint_uses(body: &GraphPattern) -> Vec<Variable> {
    enum Node<'p> {
        Pattern(&'p GraphPattern, usize),
        Expression(&'p Expression, usize),
    }
    // The `SELECT` lists entered, shared by every node below them: each node carries how
    // many of `scopes` enclose it.
    let mut scopes: Vec<&[Variable]> = Vec::new();
    let mut scope_of: Vec<usize> = Vec::new();
    let mut uses = Vec::new();
    let mut pending = vec![Node::Pattern(body, usize::MAX)];
    while let Some(node) = pending.pop() {
        match node {
            Node::Pattern(pattern, scope) => match pattern {
                GraphPattern::Service { name, .. } => {
                    if let NamedNodePattern::Variable(v) = name
                        && visible(&scopes, &scope_of, scope, v)
                    {
                        uses.push(v.clone());
                    }
                }
                GraphPattern::Project { inner, variables } => {
                    scopes.push(variables);
                    scope_of.push(scope);
                    pending.push(Node::Pattern(inner, scopes.len() - 1));
                }
                _ => {
                    crate::governor::soundness::visit_pattern_parts(pattern, &mut |part| {
                        pending.push(match part {
                            PatternPart::Child(child, _) => Node::Pattern(child, scope),
                            PatternPart::Expression(expr) => Node::Expression(expr, scope),
                        });
                        false
                    });
                }
            },
            Node::Expression(expr, scope) => {
                crate::governor::soundness::visit_expression_parts(expr, &mut |part| {
                    match part {
                        ExpressionPart::Sub(sub) => pending.push(Node::Expression(sub, scope)),
                        ExpressionPart::Exists(inner) => pending.push(Node::Pattern(inner, scope)),
                        ExpressionPart::Call(_) => {}
                    }
                    false
                });
            }
        }
    }
    uses
}

/// Whether every `SELECT` list enclosing a node in scope `scope` names `variable`.
/// `scope` indexes `scopes`; `scope_of[i]` is the scope enclosing scope `i`, and
/// `usize::MAX` is the body itself.
fn visible(scopes: &[&[Variable]], scope_of: &[usize], scope: usize, variable: &Variable) -> bool {
    let mut at = scope;
    while at != usize::MAX {
        if !scopes[at].contains(variable) {
            return false;
        }
        at = scope_of[at];
    }
    true
}

#[cfg(test)]
thread_local! {
    /// Test-only: substitute nested `EXISTS` bodies in full, as SPARQL's `substitute`
    /// reads and as this evaluator did before bodies were deferred — the reference a
    /// differential test compares the deferred evaluation against. Thread-local, so it
    /// reaches only a test that runs its query sequentially on its own thread.
    static FORCE_EAGER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Substitute every nested `EXISTS` body in full on this thread until the guard drops.
#[cfg(test)]
pub(crate) fn force_eager_substitution_for_test() -> ForceEagerGuard {
    ForceEagerGuard {
        previous: FORCE_EAGER.with(|cell| cell.replace(true)),
    }
}

/// Whether [`force_eager_substitution_for_test`] is in force on this thread.
#[cfg(test)]
pub(crate) fn eager_forced() -> bool {
    FORCE_EAGER.with(std::cell::Cell::get)
}

/// Restores the previous [`force_eager_substitution_for_test`] setting on drop.
#[cfg(test)]
pub(crate) struct ForceEagerGuard {
    previous: bool,
}

#[cfg(test)]
impl Drop for ForceEagerGuard {
    fn drop(&mut self) {
        FORCE_EAGER.with(|cell| cell.set(self.previous));
    }
}
