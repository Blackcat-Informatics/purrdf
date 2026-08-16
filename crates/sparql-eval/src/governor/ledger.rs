// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The per-node charge ledger: where every unit of fuel went, which node committed which
//! rows, and — where the planner produced a number — what it predicted beside what
//! happened.
//!
//! # Why a ledger at all
//!
//! [`GovernorEvidence`](purrdf_core::GovernorEvidence) reports one number per dimension
//! for a whole execution. That is the right shape for *sizing* a budget and the wrong
//! shape for *understanding* one: "this query cost 4.2 million fuel" does not say which
//! operator spent it, and a caller whose budget is too small has no way to tell a
//! genuinely expensive query from a mis-planned one. The ledger is the per-node
//! decomposition of that single number, and [`QueryExplanation`] is how it is read.
//!
//! It is also the only place the cost planner's error becomes observable. `bgp`'s
//! cost-based join order minimises an *estimated* intermediate cardinality; nothing
//! previously compared that estimate to the cardinality that actually materialised. A
//! ledger row carries both, so an estimator that is wrong by three orders of magnitude
//! says so in the EXPLAIN output rather than only in a query's wall time.
//!
//! # Determinism
//!
//! Same query, same data, same budget ⇒ byte-identical ledger. Three properties buy that:
//!
//! 1. **Node identity is positional, not addressed.** A node's ordinal is its position in
//!    [`walk_spine`](super::soundness::walk_spine)'s pre-order over the plan, which is a
//!    pure function of the algebra. Addresses are used only to *find* that ordinal during
//!    evaluation and never appear in output.
//! 2. **The totals are sums, and addition commutes.** A forked worker charges the same
//!    node its parent was charging (the node cursor is copied into the fork, not reset),
//!    so the per-node total is independent of how the work was split across workers even
//!    though the interleaving is not.
//! 3. **Nothing here reads a clock.** The deadline governor is the one clock reader in the
//!    evaluator and it stays that way, which is also what keeps this
//!    `wasm32-unknown-unknown`-clean.
//!
//! # What is *not* in the ledger
//!
//! Charges an ungoverned execution never made. The ledger records what the schedule
//! charged, so it fills only when the corresponding dimension is engaged — which is why
//! [`NativeSparqlEngine::explain_query`](crate::NativeSparqlEngine::explain_query) runs
//! under [`QueryGovernors::METERED`](super::QueryGovernors::METERED): every counter
//! engaged, at a ceiling nothing can reach.
//!
//! # Attribution of work that is not a plan node
//!
//! An expression-embedded `EXISTS` re-enters whole-pattern evaluation, and a correlated
//! one does so over a *substituted temporary* tree that is allocated per outer row and
//! dropped at the end of it. Those nodes are not in the plan, so they have no ordinal. The
//! cursor therefore only moves when a node is a **known plan node**, and everything else
//! accrues to the nearest enclosing one — the `FILTER` or `BIND` that owns the expression.
//! That is what makes the ledger's fuel total equal the evidence's fuel total exactly,
//! rather than approximately — a decomposition that did not add up would be a
//! decomposition of some other number. The governed-query surface tests check it by
//! summing every ledger line's fuel column against
//! [`GovernorEvidence::consumed_in`](purrdf_core::GovernorEvidence::consumed_in) for
//! [`ResourceDimension::Fuel`](purrdf_core::ResourceDimension::Fuel).

use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_sparql_algebra::GraphPattern;

use super::soundness::{pattern_label, walk_spine};
use super::{CHARGE_SCHEDULE, ChargePoint};
use crate::DetHashMap;
use crate::agg_fn::AggDescriptor;
use crate::property_fn::PfDescriptor;

/// The cost planner's prediction for one basic graph pattern, recorded before evaluation
/// so the ledger can print it beside the row count that actually materialised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEstimate {
    /// The estimated size of the BGP's final output, in rows.
    pub rows: u64,
    /// The estimated size of the largest intermediate stage along the chosen join order,
    /// in rows. This is the quantity admission control refuses against.
    pub peak_rows: u64,
    /// The width of the BGP's output, in columns — the multiplier that turns a row
    /// estimate into the cell-denominated
    /// [`ResourceDimension::IntermediateCells`](purrdf_core::ResourceDimension::IntermediateCells)
    /// ceiling.
    pub columns: u64,
}

impl PlanEstimate {
    /// The estimated peak intermediate bag, in cells — the estimate compared against the
    /// caller's intermediate-cardinality ceiling at admission.
    ///
    /// Saturating: an estimate that overflows `u64` is already past any ceiling a caller
    /// can express, and wrapping it would turn the largest predicted bag into the
    /// smallest.
    #[must_use]
    pub const fn peak_cells(&self) -> u64 {
        self.peak_rows.saturating_mul(self.columns)
    }
}

/// One algebra node's static identity in the ledger.
#[derive(Debug, Clone)]
struct LedgerNode {
    /// The node's algebra variant label, from the one label table.
    label: &'static str,
    /// Depth below the plan root, for rendering the tree shape.
    depth: usize,
    /// The planner's prediction for this node, when it produced one (BGP nodes only).
    estimate: Option<PlanEstimate>,
}

/// Live, execution-local per-node charge accounting.
///
/// Shared by `Arc` with every forked worker for the same reason
/// [`GovernorState`](super::GovernorState) is: a per-worker copy would split one node's
/// charges across copies and none of them would be the total.
#[derive(Debug)]
pub(crate) struct ChargeLedger {
    /// One entry per plan node, in pre-order.
    nodes: Vec<LedgerNode>,
    /// Plan-node address to its pre-order ordinal. Addresses are stable for the immutable
    /// query algebra for as long as the plan is borrowed, which is the same discipline
    /// [`crate::eval::EvalCtx`]'s address-memoized caches already rely on.
    ordinals: DetHashMap<usize, usize>,
    /// Fuel charged per node, per [`ChargePoint`], in [`CHARGE_SCHEDULE`] order.
    fuel: Vec<[AtomicU64; CHARGE_SCHEDULE.len()]>,
    /// Rows each node committed to its own output.
    rows: Vec<AtomicU64>,
    /// The largest materialized bag each node held, in cells.
    cells: Vec<AtomicU64>,
}

impl ChargeLedger {
    /// A ledger over the plan rooted at `root`, with the planner's per-BGP predictions
    /// keyed by node address.
    pub(crate) fn for_plan(
        root: &GraphPattern,
        estimates: &DetHashMap<usize, PlanEstimate>,
    ) -> Self {
        let mut nodes = Vec::new();
        let mut ordinals = DetHashMap::default();
        walk_spine(root, &mut |node, _context, depth| {
            let address = std::ptr::from_ref(node) as usize;
            ordinals.insert(address, nodes.len());
            nodes.push(LedgerNode {
                label: pattern_label(node),
                depth,
                estimate: estimates.get(&address).cloned(),
            });
        });
        let count = nodes.len();
        Self {
            nodes,
            ordinals,
            fuel: (0..count)
                .map(|_| std::array::from_fn(|_| AtomicU64::new(0)))
                .collect(),
            rows: (0..count).map(|_| AtomicU64::new(0)).collect(),
            cells: (0..count).map(|_| AtomicU64::new(0)).collect(),
        }
    }

    /// The pre-order ordinal of the plan node at `address`, or `None` when the address is
    /// not a plan node — an `EXISTS` substituted temporary, or a SHACL-AF function body.
    pub(crate) fn ordinal_of(&self, address: usize) -> Option<usize> {
        self.ordinals.get(&address).copied()
    }

    /// The ordinal of the plan root, which is where a fresh cursor starts.
    pub(crate) const fn root_ordinal() -> usize {
        0
    }

    /// Charge `units` of fuel to `node` under `point`.
    ///
    /// `Relaxed` on both the load and the store: the counters carry no accompanying data
    /// a reader must see in order, and the value read at the end of the execution is
    /// read after every worker has been joined.
    pub(crate) fn record_fuel(&self, node: usize, point: ChargePoint, units: u64) {
        if let Some(slots) = self.fuel.get(node) {
            slots[point.schedule_index()].fetch_add(units, Ordering::Relaxed);
        }
    }

    /// Record `rows` committed to `node`'s output.
    pub(crate) fn record_rows(&self, node: usize, rows: u64) {
        if let Some(slot) = self.rows.get(node) {
            slot.fetch_add(rows, Ordering::Relaxed);
        }
    }

    /// Record `cells` as an observation of `node`'s materialized bag, keeping the largest.
    ///
    /// A maximum rather than a sum, for the same reason the
    /// [`ResourceDimension::IntermediateCells`](purrdf_core::ResourceDimension::IntermediateCells)
    /// ceiling is: the quantity is how large one bag got, not how many bags were built.
    pub(crate) fn record_cells(&self, node: usize, cells: u64) {
        if let Some(slot) = self.cells.get(node) {
            slot.fetch_max(cells, Ordering::Relaxed);
        }
    }

    /// The ledger as an ordered, owned snapshot — the form that crosses the public
    /// boundary.
    pub(crate) fn snapshot(&self) -> Vec<NodeCharges> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(ordinal, node)| NodeCharges {
                ordinal,
                depth: node.depth,
                label: node.label,
                fuel: std::array::from_fn(|slot| self.fuel[ordinal][slot].load(Ordering::Relaxed)),
                rows: self.rows[ordinal].load(Ordering::Relaxed),
                cells: self.cells[ordinal].load(Ordering::Relaxed),
                estimate: node.estimate.clone(),
            })
            .collect()
    }
}

/// One algebra node's line in the charge ledger.
///
/// Deliberately owned and free of evaluator types: a ledger outlives the execution that
/// produced it, and a caller reads it after the evaluation context — and the interned
/// scratch arena the rows lived in — is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeCharges {
    /// The node's position in the plan's pre-order walk. Stable for a given query.
    pub ordinal: usize,
    /// Depth below the plan root.
    pub depth: usize,
    /// The algebra variant this node is.
    pub label: &'static str,
    /// Fuel charged at this node, per charge point, in
    /// [`CHARGE_SCHEDULE`](super::CHARGE_SCHEDULE) order.
    pub fuel: [u64; CHARGE_SCHEDULE.len()],
    /// Rows this node committed to its own output.
    pub rows: u64,
    /// The largest materialized bag this node held, in cells (`rows * columns`).
    pub cells: u64,
    /// The planner's prediction for this node, when it made one.
    pub estimate: Option<PlanEstimate>,
}

impl NodeCharges {
    /// Fuel charged at this node under `point`.
    #[must_use]
    pub const fn fuel_at(&self, point: ChargePoint) -> u64 {
        self.fuel[point.schedule_index()]
    }

    /// Total fuel charged at this node, across every charge point.
    ///
    /// Saturating, for the same reason every other total in the governor tier is: a
    /// ledger is a report, and a report that wraps a very large number into a very small
    /// one is worse than one that says "at least this much".
    #[must_use]
    pub fn fuel_total(&self) -> u64 {
        self.fuel
            .iter()
            .fold(0_u64, |sum, units| sum.saturating_add(*units))
    }
}

/// A query's plan, its cost, and the identity of the schedule that priced it.
///
/// Returned by
/// [`NativeSparqlEngine::explain_query`](crate::NativeSparqlEngine::explain_query). Every
/// field is owned and deterministic; [`Self::render`] turns it into the stable text form.
#[derive(Debug, Clone)]
pub struct QueryExplanation {
    /// The charge-schedule profile this build implements — the identifier, its version,
    /// and the content address of the schedule itself.
    profile: ProfileIdentity,
    /// The cost-based BGP join orders, one string per triple pattern, in the order the
    /// planner selected — exactly what this API returned before the ledger existed.
    join_orders: Vec<String>,
    /// One line per algebra node, in the plan's pre-order.
    ledger: Vec<NodeCharges>,
    /// The full self-description of every property-function relation that was in
    /// scope, sorted by IRI.
    ///
    /// Part of the receipt because a relation is HOST code that produces rows no index
    /// sized and no dataset holds: two runs of the same query text over the same dataset
    /// can differ in nothing but which relations were injected, and an explanation that
    /// did not name them would present those two runs as the same run. The full
    /// descriptor rather than the bare IRI, because the IRI alone is not what a relation
    /// IS: two impls registered under the SAME IRI with different arity, declared modes,
    /// or volatility answer differently (and, for volatility, run under a different
    /// parallel-safety classification), and a receipt that recorded only the IRI would
    /// print the same bytes for both. Sorted so the list is a function of what was
    /// registered rather than of registration order — see
    /// [`PropertyFunctionRegistry::describe`](crate::property_fn::PropertyFunctionRegistry::describe),
    /// which this is taken from verbatim.
    relations: Vec<PfDescriptor>,
    /// The full self-description of every custom-aggregate that was in scope, sorted by
    /// IRI. The exact twin of [`Self::relations`], for the exact same reason: a `Custom`
    /// aggregate is HOST code that folds a `GROUP BY` group into a value no built-in
    /// accumulator produces, and two runs of the same query text that differ only in
    /// which aggregates were injected are two different runs — an explanation that did
    /// not name them would print the same bytes for both. Sorted by
    /// [`AggregateRegistry::describe`](crate::agg_fn::AggregateRegistry::describe), which
    /// this is taken from verbatim, so the list is a function of what was registered
    /// rather than of registration order.
    aggregates: Vec<AggDescriptor>,
    /// The whole execution's consumption and ceilings, so a reader can check the ledger's
    /// fuel column against the total it decomposes.
    evidence: purrdf_core::GovernorEvidence,
}

/// The pinned identity of the charge schedule an explanation was priced under.
///
/// Carried on every explanation because a cost is meaningless without it: two builds that
/// disagree about what a query costs must not produce two explanations that look
/// comparable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileIdentity {
    /// [`GOVERNOR_PROFILE_ID`](super::GOVERNOR_PROFILE_ID).
    pub id: &'static str,
    /// [`GOVERNOR_PROFILE_VERSION`](super::GOVERNOR_PROFILE_VERSION).
    pub version: u32,
    /// [`GOVERNOR_PROFILE_DIGEST`](super::GOVERNOR_PROFILE_DIGEST).
    pub digest: String,
    /// [`STOP_POLL_FUEL`](super::STOP_POLL_FUEL) — the fuel interval at which a host stop
    /// signal is polled, and therefore the granularity at which a deadline can be
    /// observed. Part of the identity because it bounds how far past a deadline an
    /// execution priced by this schedule can run.
    pub stop_poll_fuel: u64,
}

impl ProfileIdentity {
    /// This build's profile identity.
    #[must_use]
    pub fn current() -> Self {
        Self {
            id: super::GOVERNOR_PROFILE_ID,
            version: super::GOVERNOR_PROFILE_VERSION,
            digest: super::GOVERNOR_PROFILE_DIGEST.clone(),
            stop_poll_fuel: super::STOP_POLL_FUEL,
        }
    }
}

impl QueryExplanation {
    /// Assemble an explanation from the parts the engine computed.
    pub(crate) fn new(
        join_orders: Vec<String>,
        ledger: Vec<NodeCharges>,
        relations: Vec<PfDescriptor>,
        aggregates: Vec<AggDescriptor>,
        evidence: purrdf_core::GovernorEvidence,
    ) -> Self {
        Self {
            profile: ProfileIdentity::current(),
            join_orders,
            ledger,
            relations,
            aggregates,
            evidence,
        }
    }

    /// The charge schedule this explanation was priced under.
    #[must_use]
    pub const fn profile(&self) -> &ProfileIdentity {
        &self.profile
    }

    /// The cost-based BGP join orders: for every BGP with at least two triple patterns,
    /// its patterns in the order the planner selected.
    ///
    /// This is exactly the value this API returned before the ledger existed, kept as a
    /// borrowed slice so a caller that only wanted the join order is unaffected.
    #[must_use]
    pub fn join_orders(&self) -> &[String] {
        &self.join_orders
    }

    /// One line per algebra node, in the plan's pre-order.
    #[must_use]
    pub fn ledger(&self) -> &[NodeCharges] {
        &self.ledger
    }

    /// The full self-description of every property-function relation that was in
    /// scope, sorted by IRI.
    ///
    /// Empty when the explanation was taken with no registry injected, which is the same
    /// thing an empty registry means: no predicate in the query could resolve to a
    /// relation.
    #[must_use]
    pub fn relations(&self) -> &[PfDescriptor] {
        &self.relations
    }

    /// The full self-description of every custom-aggregate that was in scope, sorted by
    /// IRI.
    ///
    /// Empty when the explanation was taken with no aggregate registry injected, which is
    /// the same thing an empty registry means: no `AGG(<iri>, …)` call in the query could
    /// resolve to a registered aggregate. Populated by
    /// [`NativeSparqlEngine::explain_query_with_aggregates`](crate::NativeSparqlEngine::explain_query_with_aggregates),
    /// [`NativeSparqlEngine::explain_query_with_aggregates_view`](crate::NativeSparqlEngine::explain_query_with_aggregates_view),
    /// the exact counterpart [`Self::relations`] has for the property-function seam.
    #[must_use]
    pub fn aggregates(&self) -> &[AggDescriptor] {
        &self.aggregates
    }

    /// The whole execution's consumption and ceilings.
    #[must_use]
    pub const fn evidence(&self) -> &purrdf_core::GovernorEvidence {
        &self.evidence
    }

    /// The stable text rendering: a profile header, the charge schedule, the per-node
    /// ledger, the injected relations, the injected custom aggregates, and the join
    /// orders.
    ///
    /// Byte-deterministic for a given query, dataset, and build — every number in it is a
    /// counter and every string is either a pinned schedule label or an algebra variant
    /// label. There is no clock, no address, and no iteration over a hash map.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;

        let mut out = String::new();
        let _ = writeln!(
            out,
            "profile {} v{} digest {} stop-poll-fuel {}",
            self.profile.id, self.profile.version, self.profile.digest, self.profile.stop_poll_fuel
        );
        let _ = writeln!(out, "schedule");
        for (label, cost) in CHARGE_SCHEDULE {
            let _ = writeln!(out, "  {label}\t{cost}");
        }
        let _ = writeln!(out, "ledger");
        for node in &self.ledger {
            let indent = "  ".repeat(node.depth + 1);
            let _ = write!(
                out,
                "{indent}#{ordinal} {label} fuel={fuel} rows={rows} cells={cells}",
                ordinal = node.ordinal,
                label = node.label,
                fuel = node.fuel_total(),
                rows = node.rows,
                cells = node.cells,
            );
            if let Some(estimate) = &node.estimate {
                let _ = write!(
                    out,
                    " estimated-rows={} actual-rows={} estimated-peak-cells={}",
                    estimate.rows,
                    node.rows,
                    estimate.peak_cells(),
                );
            }
            out.push('\n');
            for point in ChargePoint::ALL {
                let units = node.fuel_at(point);
                if units != 0 {
                    let _ = writeln!(out, "{indent}  {}\t{units}", point.label());
                }
            }
        }
        // Always emitted, empty or not: a block that appears only when something was
        // registered would make "no relations were in scope" and "this build does not
        // report relations" the same bytes.
        let _ = writeln!(out, "relations");
        for descriptor in &self.relations {
            let _ = write!(
                out,
                "  {} arity={},{} volatility={}",
                descriptor.iri,
                descriptor.subject_arity,
                descriptor.object_arity,
                descriptor.volatility.label(),
            );
            for mode in &descriptor.modes {
                let _ = write!(out, " {}={}", mode.code, mode.rows_per_invocation);
            }
            out.push('\n');
        }
        // Always emitted, empty or not, for the exact reason the `relations` block is: see
        // above.
        let _ = writeln!(out, "aggregates");
        for descriptor in &self.aggregates {
            let _ = write!(
                out,
                "  {} arity={} volatility={} algebraic-class={} state-bound={}",
                descriptor.iri,
                descriptor.arity,
                descriptor.volatility.label(),
                descriptor.algebraic_class.label(),
                descriptor.state_bound,
            );
            for spec in &descriptor.scalarvals {
                let _ = write!(out, " {}={}", spec.name, spec.kind.label());
            }
            out.push('\n');
        }
        let _ = writeln!(out, "join-orders");
        for pattern in &self.join_orders {
            let _ = writeln!(out, "  {pattern}");
        }
        let _ = writeln!(out, "consumed");
        for dimension in purrdf_core::ResourceDimension::ALL {
            let _ = writeln!(
                out,
                "  {}\t{}",
                dimension.label(),
                self.evidence.consumed_in(dimension)
            );
        }
        out
    }
}

impl std::fmt::Display for QueryExplanation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}
