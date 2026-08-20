// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A first-party, CLOSED set of statistical aggregates, delivered through the
//! [`crate::agg_fn`] custom-aggregate seam under a **caller-configured**
//! namespace — never a purrdf-owned vocabulary (see the crate's "mints no
//! vocabulary IRIs" contract). [`AggregateRegistry::register_statistical_aggregates`]
//! is the single entry point: a Rust host that already constructs an
//! [`AggregateRegistry`] (for [`crate::engine::QueryOptions::aggregates`]) calls it
//! once with a namespace IRI, and every member below becomes reachable from the
//! query text as `AGG(<{NAMESPACE}LOCAL-NAME>, args…)` — a Python/WASM/C/CLI
//! binding that already exposes registry configuration gets the whole set for
//! free, through the string surface, with zero additional Rust callbacks.
//!
//! # The shipped set
//!
//! `MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
//! `FIRST`, `LAST`, `TOPK` — ten members, closed (no caller extension point; a
//! local name outside this set under the configured namespace is registered
//! nowhere, so `AGG(<{NS}NOT_A_MEMBER>, ?x)` is refused at prepare time exactly as
//! any other unregistered custom-aggregate IRI is).
//!
//! # Numeric discipline
//!
//! Every numeric member works over [`purrdf_xsd::XsdValue`]'s promotion tower
//! (`integer ⊂ decimal ⊂ float ⊂ double`), through the SAME `numeric_add`/
//! `numeric_sub`/`numeric_mul`/`numeric_div`/`numeric_floor` primitives
//! `crate::modifier`'s built-in `SUM`/`AVG` fold uses — a mixed integer/decimal
//! group promotes to decimal exactly the way `AVG(1, 0.5)` already does, and
//! division rounds at the same 18-fractional-digit ceiling `AVG` already accepts.
//! A non-numeric input to a numeric member **poisons** the fold to unbound —
//! `SUM`'s own discipline (see `crate::modifier`'s "Aggregate error handling"
//! docs) — rather than raising a hard error, so one malformed row does not abort
//! the whole query over an otherwise-honest aggregate.
//!
//! `STDDEV`/`STDDEV_POP` take a square root, which is irrational for almost
//! every input and therefore cannot stay in the exact decimal tower the way
//! `VARIANCE`/`VAR_POP` do: the variance itself is computed exactly (promoted
//! per the tower above), and ONLY the final `sqrt` step drops to `xsd:double`
//! (Rust's `f64::sqrt`), mirroring how this crate already treats `xsd:double`
//! as its one inexact numeric tier. `VARIANCE`/`VAR_POP` never leave the exact
//! tower.
//!
//! `STDDEV`/`VARIANCE` are the **sample** statistics (`n - 1` denominator,
//! undefined — unbound — for a group of fewer than 2 values);
//! `STDDEV_POP`/`VAR_POP` are the **population** statistics (`n` denominator,
//! defined, `0`, for a single-value group) — the same naming convention SQL's
//! `STDDEV`/`STDDEV_POP` pair uses.
//!
//! `MEDIAN` is defined as `PERCENTILE` at `p = 0.5` under **linear
//! interpolation between the two closest ranks** (`rank = p × (n − 1)`,
//! interpolating between `⌊rank⌋` and `⌈rank⌉`): for an even-sized group this
//! reduces EXACTLY to the arithmetic mean of the two middle values (the
//! standard reading), computed in the promoted numeric type of those two
//! values — a decimal `0.5` literal keeps the whole computation exact. `p = 0`
//! and `p = 1` reduce to the group's minimum and maximum.
//!
//! # The `xsd:duration` extension
//!
//! `MEDIAN`, `PERCENTILE`, `MODE`, `FIRST`, `LAST`, `TOPK` also accept the
//! `xsd:duration` group (`xsd:duration`/`xsd:yearMonthDuration`/
//! `xsd:dayTimeDuration` — one value space per XSD 1.1 Part 2 §3.3.6, all
//! represented by [`purrdf_xsd::XsdValue::Duration`]) — this crate's
//! SEP-0002 temporal-arithmetic surface ([`purrdf_xsd::value_add`]/
//! [`purrdf_xsd::value_sub`]/[`purrdf_xsd::value_mul`]/
//! [`purrdf_xsd::value_cmp`]) extends the aggregate algebra to durations the
//! same way `crate::modifier`'s `SUM`/`AVG` already do (see that module's
//! "`SUM`/`AVG` over `xsd:duration`" doc section) — a PurRDF extension
//! beyond F&O, which defines none of these over `xsd:duration`.
//!
//! * `MODE`/`FIRST`/`LAST`/`TOPK` needed NO code change at all: they already
//!   fold over a [`TermValue`] of ANY kind (see each member's own doc
//!   section above), gated by nothing numeric-specific — none of their
//!   `step` implementations ever call `is_numeric_xsd`. `MODE`'s tie-break
//!   and `TOPK`'s total order are both `crate::modifier::term_value_order`,
//!   which already orders duration literals correctly (through
//!   `literal_order`'s `parse_by_iri` + `value_cmp` path) — see the
//!   ordering-policy paragraph below for the one difference between that
//!   order and the one `MEDIAN`/`PERCENTILE` use.
//! * `MEDIAN`/`PERCENTILE` (order statistics needing interpolation) widen
//!   their gate from `is_numeric_xsd` alone to "numeric OR duration, never
//!   both in the same group" (see `is_numeric_or_duration_xsd`/
//!   `same_value_family`) — a mixed numeric+duration group POISONS, the
//!   same discipline `crate::modifier`'s duration `SUM`/`AVG` extension
//!   already uses for the identical mixed case. The interpolation step
//!   (`percentile_of`'s `diff`/`scaled`/final sum) now goes through
//!   [`purrdf_xsd::value_sub`]/[`purrdf_xsd::value_mul`]/
//!   [`purrdf_xsd::value_add`] rather than the numeric-tower-only
//!   `numeric_sub`/`numeric_mul`/`numeric_add` — a behavior-preserving
//!   substitution for numeric groups (`value_*` dispatches straight to
//!   `numeric_*` for numeric operands) that ALSO accepts `DUR − DUR`/
//!   `DUR × exact-decimal-fraction`/`DUR + DUR`. `p` itself (`MEDIAN`'s
//!   fixed `0.5`, `PERCENTILE`'s named `P`) and the `rank`/`floor`/
//!   `fraction` derived from it stay on the numeric tower unconditionally
//!   — `p` is a proportion of a COUNT, never a duration, regardless of what
//!   the series holds. A mixed-SUBTYPE duration group
//!   (`yearMonthDuration` values beside `dayTimeDuration` ones) is NOT
//!   mixed-FAMILY, so it does not poison; its interpolated result's
//!   datatype follows the SAME "both dt → dt, both ym → ym, else general"
//!   join [`purrdf_xsd::temporal::add_durations`]/`subtract_durations`/
//!   `multiply_duration` already apply internally — reused as-is through
//!   the public `value_*` surface above, never re-derived here.
//!
//! ## Ordering policy for incomparable duration pairs
//!
//! `P1M` (`yearMonthDuration`) and `P30D` (`dayTimeDuration`) are
//! VALUE-INCOMMENSURABLE — [`purrdf_xsd::value_cmp`] returns `None` for that
//! pair (see its own docs), exactly as it does for a `NaN` numeric
//! comparison. `MEDIAN`/`PERCENTILE`'s sort (inside `finish`, before
//! ranking) already had a policy for THIS exact shape of question before
//! durations existed: `numeric_cmp(a, b).unwrap_or(Ordering::Equal)` — an
//! incomparable pair sorts as EQUAL (stably retaining relative input
//! order), not through a secondary deterministic tie-break key. Widening the
//! comparator's SOURCE from `numeric_cmp` to [`purrdf_xsd::value_cmp`]
//! (needed so duration pairs compare at all) keeps that SAME
//! `.unwrap_or(Ordering::Equal)` policy unchanged — a deliberate
//! CONSISTENCY choice, not the only possible one: `MIN`/`MAX`/`MODE`/`TOPK`
//! (`crate::modifier::fold_extreme`/`term_value_order`) instead fall back to
//! a genuine total order (further tie-broken by `(datatype, language,
//! lexical form)`), because THEIR existing policy, before durations, was
//! already that total order, not "treat as equal" — each aggregate mirrors
//! ITS OWN prior policy rather than adopting the other's, per this crate's
//! "consistency with the existing surface" rule.
//!
//! # Why `STDDEV`/`STDDEV_POP`/`VARIANCE`/`VAR_POP` stay numeric-only
//!
//! The running-moments fold needs `x²` (`moments_step`'s `numeric_mul(x,
//! x)`) — `xsd:duration × xsd:duration` is NOT in the SEP-0002 value space
//! ([`purrdf_xsd::value_mul`]'s docs: a duration only multiplies by an EXACT
//! numeric factor, never by another duration; "seconds²" or "months²" has no
//! XSD datatype to hold it). There is no well-defined "duration variance" to
//! fall back to without inventing a value space this crate does not own, so
//! this family is deliberately NOT extended — a duration input to
//! `STDDEV`/`VARIANCE` poisons the fold exactly as any other non-numeric
//! value does (the unchanged `is_numeric_xsd` gate in
//! `MomentsAccumulator::step`).
//!
//! # `PERCENTILE`'s named scalarval
//!
//! `AGG(<{NS}PERCENTILE>, ?x; P=0.95)` — `P` is a NAMED SCALARVAL (see
//! [`purrdf_sparql_algebra::AggregateExpression::scalarvals`]'s docs and
//! [`crate::agg_fn::CustomAggregate::scalarvals`]), not a positional argument:
//! ONE value for the whole aggregation, resolved once at accumulator `init`
//! time, never re-evaluated per row. This is a deliberate correction from an
//! earlier shape that took `p` as a second positional argument
//! (`AGG(<{NS}PERCENTILE>, ?x, ?p)`) — semantically wrong, since a positional
//! argument is evaluated PER ROW, and `p` is a parameter of the aggregation
//! itself, not a per-row quantity. `crate::property_fn_plan::plan_aggregate`
//! refuses a call missing `P`, naming an unrecognized scalarval, or supplying
//! a non-numeric `P` at PREPARE time, before any evaluation work is spent. `P`
//! outside `[0, 1]` still poisons the fold to unbound at EVALUATION time
//! (never a hard error — the SAME "poison, don't abort" discipline every
//! other numeric member uses), because that check depends on nothing prepare
//! time can see (a literal `P=1.5` is a well-typed decimal; only its VALUE is
//! out of domain).
//!
//! # `MODE`
//!
//! Works over **any** term kind (it counts term occurrences by RDF term
//! identity — [`purrdf_core::TermValue`]'s own `Eq`, not "value equality":
//! `"5"^^xsd:integer` and `"05"^^xsd:integer` are numerically equal but are
//! counted as two DIFFERENT terms, exactly as `DISTINCT`'s own dedup treats
//! them). A tie among several terms with the same maximum count is broken by
//! the smallest term under the SAME total order `MIN`/`ORDER BY` use
//! (`crate::modifier::term_value_order`); a further tie (distinct terms
//! that compare value-equal under that order, e.g. `"5"`/`"05"`) falls back to
//! [`purrdf_core::TermValue`]'s own canonical structural order, so the choice
//! is fully deterministic regardless of scan order.
//!
//! # `FIRST`/`LAST`
//!
//! Work over any term kind, in **input row order** (the same row order
//! `GROUP_CONCAT`/`SAMPLE` fold over — see `crate::modifier`'s module docs):
//! `FIRST` is the earliest row's value, `LAST` the latest.
//!
//! # `TOPK`'s named scalarval and its "one term" contract
//!
//! `AGG(<{NS}TOPK>, ?x; K=3)` — `K`, like `PERCENTILE`'s `P`, is a NAMED
//! SCALARVAL, not a positional argument: a positive `xsd:integer`, ONE value
//! for the whole aggregation, resolved once at `init` time (the same
//! correction from an earlier two-positional-argument shape `PERCENTILE`'s
//! docs describe — `K` is a parameter of the aggregation, not a per-row
//! quantity). A missing, non-integer, or non-positive `K` poisons the fold to
//! unbound (never a hard error, the same "poison, don't abort" discipline
//! every other numeric member uses) — `crate::property_fn_plan::plan_aggregate`
//! catches "missing" and "not an `xsd:integer` literal" at prepare time; only
//! "present, well-typed, but `≤ 0`" survives to evaluation, exactly as `P`
//! outside `[0, 1]` does for `PERCENTILE`. Every SPARQL aggregate's contract
//! is to yield exactly ONE RDF term (this crate has no CDT-list container
//! type to hand back "the k values" as a list), so `TOPK` answers the way
//! `GROUP_CONCAT` already answers an inherently multi-valued question: the
//! top `k` values, in DESCENDING order under the SAME total value order
//! `MIN`/`MAX`/`MODE` use, with their **lexical forms** joined by a single
//! fixed space separator (mirroring `crate::modifier`'s `GROUP_CONCAT`, whose
//! default separator is likewise `" "` per SPARQL §18.6.1.7). A configurable
//! separator would need a second named scalarval; `TOPK(value; K=k)` keeps
//! the scalarval contract as simple as `PERCENTILE`'s, so the separator is
//! fixed by design, not by oversight. A term with no lexical form (a blank
//! node or a triple term — the same case `GROUP_CONCAT` itself poisons on)
//! poisons `TOPK` too. `TOPK` retains DUPLICATES — it counts rows, not
//! distinct values, the same multiset reading `insert_bounded` folds under —
//! so fewer than `k` rows in the group (whether or not any repeat) → every
//! value the group has, in row-multiplicity. Empty group → unbound, like
//! every other member here.
//!
//! # Real merges via `AggregateAccumulator::into_any` (all ten are `Volatility::Stable`)
//!
//! [`crate::agg_fn::AggregateAccumulator::combine`] receives `other` as a fully
//! TYPE-ERASED `Box<dyn AggregateAccumulator>`. Naively, the only observable
//! surface through it is `other.finish()` (the pattern this crate's own
//! fixtures use for `SUM`/list-collection — see `crate::agg_fn`'s
//! `SumAccumulator` test and `crate::modifier`'s `ListCollector` test), which
//! is sound EXACTLY when a fold's `finish()` output is *itself* sufficient
//! mergeable state — true for `SUM` (a running total IS the raw state) and
//! `GROUP_CONCAT`/list-collection (a joined string IS the raw state, losslessly
//! re-splittable). It is NOT true for a statistic whose finished, single-term
//! answer throws away the information a correct merge needs: two partial
//! standard deviations cannot be combined into the whole group's standard
//! deviation without each side's count and sum-of-squares too; two partial
//! modes cannot be combined without each side's raw value multiset, which a
//! single winning term does not carry; two partial medians/percentiles/top-`k`
//! sets cannot be combined from their single finished answer at all.
//!
//! [`crate::agg_fn::AggregateAccumulator::into_any`] is the trait's escape
//! hatch for exactly this: it recovers `other`'s original concrete type — same
//! type BY CONSTRUCTION, since every partial accumulator a `combine` chain
//! ever merges was created by the SAME [`crate::agg_fn::CustomAggregate::init`]
//! factory — so `combine` can merge the SAME structural state `step` builds,
//! through `crate::agg_fn::downcast_combine_partial`, rather than a lossy
//! re-derivation from `finish()`. Each member below uses it:
//!
//! * `MEDIAN`/`PERCENTILE` merge their (unsorted-until-`finish`) value lists by
//!   concatenation — merge order does not matter, since `finish` sorts before
//!   computing a rank either way; `PERCENTILE` additionally poisons the merge
//!   if the two sides disagree on `p`, exactly as `step` poisons on a
//!   within-accumulator mismatch.
//! * `MODE` merges its raw value multiset the same way (concatenation):
//!   `finish`'s sort + run-length scan recovers each value's count from
//!   whatever multiset it is handed, so concatenating two partials' lists IS
//!   "sum the counts per value" — no explicit count map is needed to get that
//!   effect.
//! * `STDDEV`/`STDDEV_POP`/`VARIANCE`/`VAR_POP` merge their `(n, Σx, Σx²)`
//!   moments componentwise (`n + n'`, `Σx + Σx'`, `Σx² + Σx'²`) — exact, no
//!   precision loss, for the same reason the running fold itself is exact (see
//!   this family's own doc comment below).
//! * `TOPK` merges by inserting one side's values into the other's bounded
//!   top-`k` set and truncating (`insert_bounded`, the same primitive `step`
//!   itself uses) — a bounded-structure merge, never `O(group size)`.
//!
//! Every one of these merges is a genuinely deterministic fold with no
//! dependency on WHICH worker produced which partial or in what order two
//! partials of equal size are combined (see each member's [`AlgebraicClass`]
//! for the precise law) — these eight were never actually nondeterministic,
//! only architecturally unable to merge through the finish-only path. All ten
//! members (these eight plus `FIRST`/`LAST`) therefore declare
//! `Volatility::Stable`, the honest classification for a fully deterministic
//! fold, and are eligible for `crate::modifier::eval_custom_aggregate`'s
//! within-group chunked fold exactly like any other `Stable` custom aggregate.
//!
//! `FIRST`/`LAST` are the two members whose `finish()` output (the earliest
//! or latest value seen SO FAR) genuinely is sufficient state on its own — an
//! earlier chunk's first value stays the group's first value no matter what a
//! later chunk saw, and symmetrically for `LAST` — so both merge through
//! `finish()`, needing no downcast.
//!
//! # `state_bound` honesty
//!
//! [`crate::agg_fn::CustomAggregate::state_bound`] is a FIXED, per-aggregate
//! declaration — it takes no group-size parameter, so it cannot literally
//! bound a fold whose real memory is proportional to its group's cardinality
//! (`MEDIAN`/`PERCENTILE`/`MODE`, which retain the whole group) or to a
//! per-call argument the declaration is read before any row is seen
//! (`TOPK`'s `k`). The running-moments accumulators (`STDDEV`/`STDDEV_POP`/
//! `VARIANCE`/`VAR_POP` — see that section below for why they are `(n, Σx,
//! Σx²)`, not Welford's `(n, mean, M2)`) are the one family that IS genuinely
//! `O(1)`: three
//! stack-only [`purrdf_xsd::XsdValue`] numeric fields (`Integer`/`Decimal`/
//! `Float`/`Double` never heap-allocate) plus a row count, regardless of
//! group size. The rest declare a nominal, documented estimate rather than a
//! true worst case — the SAME convention this crate's own
//! `ListCollectorAggregate` test fixture already uses (a flat `256` despite
//! folding thousands of rows) — because true adversarial growth is bounded
//! ANYWAY by [`crate::governor::ChargePoint::AggregateAccumulation`]'s
//! per-row `Fuel` charge, a SEPARATE governed dimension from `ScratchBytes`
//! that already prices the number of rows any single group may fold.
//!
//! # `DISTINCT`
//!
//! Handled entirely by the evaluator before an accumulator's `step` ever
//! runs (see `crate::modifier::eval_custom_aggregate`'s phase-1 tuple dedup)
//! — nothing in this module special-cases it.

use std::cmp::Ordering;
use std::mem;
use std::sync::Arc;

use purrdf_core::TermValue;
use purrdf_xsd::numeric::numeric_cmp;
use purrdf_xsd::{
    XsdDatatype, XsdValue, numeric_add, numeric_div, numeric_floor, numeric_mul, numeric_sub,
    value_add, value_cmp, value_mul, value_sub,
};

use crate::agg_fn::{
    AggregateAccumulator, AggregateRegistry, AlgebraicClass, CustomAggregate, ScalarvalKind,
    ScalarvalSpec, downcast_combine_partial,
};
use crate::error::EvalError;
use crate::expr::xsd_of;
use crate::modifier::{is_numeric_xsd, lexical_of, term_value_order};
use crate::user_fn::{Arity, Volatility};

// ---------------------------------------------------------------------------
// Local names (the closed set) and shared constants
// ---------------------------------------------------------------------------

const MEDIAN: &str = "MEDIAN";
const PERCENTILE: &str = "PERCENTILE";
const STDDEV: &str = "STDDEV";
const STDDEV_POP: &str = "STDDEV_POP";
const VARIANCE: &str = "VARIANCE";
const VAR_POP: &str = "VAR_POP";
const MODE: &str = "MODE";
const FIRST: &str = "FIRST";
const LAST: &str = "LAST";
const TOPK: &str = "TOPK";

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

/// `PERCENTILE`'s named scalarval: `AGG(<{NS}PERCENTILE>, ?v; P=0.95)`.
const PERCENTILE_P: &str = "P";
/// `TOPK`'s named scalarval: `AGG(<{NS}TOPK>, ?v; K=3)`.
const TOPK_K: &str = "K";

/// Look up a named scalarval by (already upper-cased) key in the resolved
/// `(name, value)` slice [`CustomAggregate::init`] receives, filtered to the
/// SPARQL numeric tower. `None` covers BOTH "absent" and "present but
/// non-numeric" — both poison the fold the same way (see each member's `init`),
/// which is the correct behavior even though prepare-time validation
/// (`crate::property_fn_plan::plan_aggregate`) should already have refused a
/// non-numeric value or a missing required name before evaluation is ever
/// reached; this is the same defense-in-depth `eval_custom_aggregate`'s own
/// doc comment describes for a caller that bypasses that walk.
fn numeric_scalarval(scalarvals: &[(String, TermValue)], name: &str) -> Option<XsdValue> {
    scalarvals
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| xsd_of(v))
        .filter(is_numeric_xsd)
}

/// `MEDIAN`/`PERCENTILE`'s widened gate — see the module docs' "The
/// `xsd:duration` extension" section: the SPARQL numeric tower OR the
/// `xsd:duration` value space, never judged relative to anything else
/// already in the group (that check is [`same_value_family`]'s job, applied
/// separately in `step`/`combine`).
fn is_numeric_or_duration_xsd(v: &XsdValue) -> bool {
    is_numeric_xsd(v) || matches!(v, XsdValue::Duration(_))
}

/// Whether two values already passed through [`is_numeric_or_duration_xsd`]
/// are in the SAME family — both numeric or both duration. `MEDIAN`/
/// `PERCENTILE` commit to the first folded value's family; any later value
/// whose family disagrees poisons the group (see the module docs) — mixed
/// SUBTYPE duration values (`yearMonthDuration` beside `dayTimeDuration`)
/// are the SAME family and never poison here.
fn same_value_family(a: &XsdValue, b: &XsdValue) -> bool {
    is_numeric_xsd(a) == is_numeric_xsd(b)
}

/// The running-moments (`(n, Σx, Σx²)`, deliberately NOT Welford — see the
/// module docs and this family's own section comment below for why) accumulators'
/// declared [`CustomAggregate::state_bound`]: genuinely `O(1)` — see the module
/// docs.
const MOMENTS_STATE_BOUND: u64 = 64;
/// `MEDIAN`/`PERCENTILE`/`MODE`'s declared [`CustomAggregate::state_bound`]: a
/// nominal, documented estimate — see the module docs' "`state_bound` honesty"
/// section.
const VALUE_PROPORTIONAL_STATE_BOUND: u64 = 1024;
/// `TOPK`'s declared [`CustomAggregate::state_bound`]: nominal, bounded by a
/// "typical" `k` — see the module docs.
const TOPK_STATE_BOUND: u64 = 512;
/// `FIRST`/`LAST`'s declared [`CustomAggregate::state_bound`]: one retained
/// [`TermValue`] clone.
const SCALAR_STATE_BOUND: u64 = 64;

/// The exact decimal `0.5`, used as `MEDIAN`'s fixed percentile parameter so its
/// whole computation stays in the exact tower (see the module docs).
fn half() -> XsdValue {
    purrdf_xsd::parse("0.5", XsdDatatype::Decimal)
        .expect("the literal \"0.5\" always parses as xsd:decimal")
}

/// Convert a numeric [`XsdValue`] to `f64` — used only by `STDDEV`/`STDDEV_POP`'s
/// final (necessarily inexact) `sqrt` step.
fn to_f64(v: &XsdValue) -> Option<f64> {
    match v {
        XsdValue::Integer { value, .. } => Some(*value as f64),
        XsdValue::Decimal(d) => Some(d.to_f64()),
        XsdValue::Float(f) => Some(f64::from(*f)),
        XsdValue::Double(d) => Some(*d),
        _ => None,
    }
}

/// Wrap a computed [`XsdValue`] into its canonical typed-literal [`TermValue`].
fn xsd_value_to_term(v: &XsdValue) -> TermValue {
    TermValue::typed_literal(v.canonical_lexical(), v.datatype().iri())
}

/// The floor of a numeric [`XsdValue`] (already passed through
/// [`numeric_floor`]) as an index-usable `i128`.
fn xsd_floor_index(v: &XsdValue) -> Option<i128> {
    match v {
        XsdValue::Integer { value, .. } => Some(*value),
        XsdValue::Decimal(d) => Some(d.whole_part()),
        XsdValue::Float(f) => Some(*f as i128),
        XsdValue::Double(d) => Some(*d as i128),
        _ => None,
    }
}

/// The `p`-th percentile of an already value-order-sorted, non-empty
/// numeric-OR-duration series (never mixed — see [`same_value_family`]),
/// under linear interpolation between the two closest ranks (see the module
/// docs). `None` (poison) when `p` is outside `[0, 1]` or any step of the
/// arithmetic fails. `rank`/`floor_v`/`fraction` are always computed on the
/// numeric tower (`p` is a proportion of a COUNT, never of the series'
/// element type); only the final interpolation (`diff`/`scaled`/the result)
/// goes through [`value_sub`]/[`value_mul`]/[`value_add`], which accept
/// BOTH the numeric tower (dispatching straight to `numeric_sub`/
/// `numeric_mul`/`numeric_add`, so numeric behavior is unchanged) and the
/// `xsd:duration` group (see the module docs' "The `xsd:duration`
/// extension" section).
fn percentile_of(sorted: &[XsdValue], p: &XsdValue) -> Option<XsdValue> {
    let zero = XsdValue::Integer {
        value: 0,
        datatype: XsdDatatype::Integer,
    };
    let one = XsdValue::Integer {
        value: 1,
        datatype: XsdDatatype::Integer,
    };
    if numeric_cmp(p, &zero)? == Ordering::Less || numeric_cmp(p, &one)? == Ordering::Greater {
        return None;
    }
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    if n == 1 {
        return Some(sorted[0].clone());
    }
    let n_minus_1 = XsdValue::Integer {
        value: i128::try_from(n - 1).ok()?,
        datatype: XsdDatatype::Integer,
    };
    let rank = numeric_mul(p, &n_minus_1).ok()?;
    let floor_v = numeric_floor(&rank).ok()?;
    let lo = xsd_floor_index(&floor_v)?.clamp(0, i128::try_from(n - 1).ok()?);
    let lo_idx = usize::try_from(lo).ok()?;
    let hi_idx = (lo_idx + 1).min(n - 1);
    if lo_idx == hi_idx {
        return Some(sorted[lo_idx].clone());
    }
    let fraction = numeric_sub(&rank, &floor_v).ok()?;
    let diff = value_sub(&sorted[hi_idx], &sorted[lo_idx]).ok()?;
    let scaled = value_mul(&diff, &fraction).ok()?;
    value_add(&sorted[lo_idx], &scaled).ok()
}

// ---------------------------------------------------------------------------
// MEDIAN
// ---------------------------------------------------------------------------

/// `MEDIAN`/`PERCENTILE`'s running (unsorted) value list — either the
/// numeric tower OR the `xsd:duration` group, NEVER mixed (see
/// [`is_numeric_or_duration_xsd`]/[`same_value_family`] and the module
/// docs' "The `xsd:duration` extension" section).
enum ValueSeries {
    Empty,
    Ok(Vec<XsdValue>),
    Poisoned,
}

struct MedianAccumulator {
    state: ValueSeries,
}

impl AggregateAccumulator for MedianAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if matches!(self.state, ValueSeries::Poisoned) {
            return Ok(());
        }
        let Some(x) = args
            .first()
            .and_then(xsd_of)
            .filter(is_numeric_or_duration_xsd)
        else {
            self.state = ValueSeries::Poisoned;
            return Ok(());
        };
        self.state = match mem::replace(&mut self.state, ValueSeries::Empty) {
            ValueSeries::Empty => ValueSeries::Ok(vec![x]),
            ValueSeries::Ok(mut values) if same_value_family(&values[0], &x) => {
                values.push(x);
                ValueSeries::Ok(values)
            }
            ValueSeries::Ok(_) => ValueSeries::Poisoned,
            ValueSeries::Poisoned => ValueSeries::Poisoned,
        };
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        // Concatenate the two (still-unsorted) value lists: merge order never
        // matters, because `finish` sorts the whole multiset before computing
        // a rank — see the module docs' "Real merges" section. A family
        // mismatch between the two partials (numeric vs duration) poisons,
        // the same as a within-accumulator mismatch in `step` above.
        let other = downcast_combine_partial::<Self>(other)?;
        self.state = match (
            mem::replace(&mut self.state, ValueSeries::Empty),
            other.state,
        ) {
            (ValueSeries::Poisoned, _) | (_, ValueSeries::Poisoned) => ValueSeries::Poisoned,
            (ValueSeries::Empty, s) | (s, ValueSeries::Empty) => s,
            (ValueSeries::Ok(mut values), ValueSeries::Ok(other_values)) => {
                if same_value_family(&values[0], &other_values[0]) {
                    values.extend(other_values);
                    ValueSeries::Ok(values)
                } else {
                    ValueSeries::Poisoned
                }
            }
        };
        Ok(())
    }

    /// See [`AggregateAccumulator::into_any`]'s trait docs — every implementor's
    /// body is this same one line.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let Self { state } = *self;
        match state {
            ValueSeries::Empty | ValueSeries::Poisoned => Ok(None),
            ValueSeries::Ok(mut values) => {
                // `value_cmp` (not `numeric_cmp`): the sort must also order
                // durations. An incomparable pair (e.g. `P1M` vs `P30D`)
                // sorts as EQUAL — the SAME policy this comparator already
                // used for an incomparable numeric pair, widened in SOURCE
                // only; see the module docs' "Ordering policy for
                // incomparable duration pairs" section.
                values.sort_by(|a, b| value_cmp(a, b).unwrap_or(Ordering::Equal));
                Ok(percentile_of(&values, &half())
                    .as_ref()
                    .map(xsd_value_to_term))
            }
        }
    }
}

struct MedianAggregate;

impl CustomAggregate for MedianAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Associative
    }
    fn state_bound(&self) -> u64 {
        VALUE_PROPORTIONAL_STATE_BOUND
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(MedianAccumulator {
            state: ValueSeries::Empty,
        })
    }
}

// ---------------------------------------------------------------------------
// PERCENTILE
// ---------------------------------------------------------------------------

struct PercentileAccumulator {
    /// `None` when `P` was missing or non-numeric at `init` time — a
    /// defense-in-depth poison for a caller that bypassed prepare-time
    /// validation (see `numeric_scalarval`'s docs); the ordinary path always
    /// has `Some` here, because `crate::property_fn_plan::plan_aggregate`
    /// already refused any call this accumulator would otherwise see with `P`
    /// missing or wrong-typed.
    p: Option<XsdValue>,
    state: ValueSeries,
}

impl AggregateAccumulator for PercentileAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if self.p.is_none() {
            self.state = ValueSeries::Poisoned;
            return Ok(());
        }
        if matches!(self.state, ValueSeries::Poisoned) {
            return Ok(());
        }
        let Some(x) = args
            .first()
            .and_then(xsd_of)
            .filter(is_numeric_or_duration_xsd)
        else {
            self.state = ValueSeries::Poisoned;
            return Ok(());
        };
        self.state = match mem::replace(&mut self.state, ValueSeries::Empty) {
            ValueSeries::Empty => ValueSeries::Ok(vec![x]),
            ValueSeries::Ok(mut values) if same_value_family(&values[0], &x) => {
                values.push(x);
                ValueSeries::Ok(values)
            }
            ValueSeries::Ok(_) => ValueSeries::Poisoned,
            ValueSeries::Poisoned => ValueSeries::Poisoned,
        };
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        // `p` is identical on both sides BY CONSTRUCTION — every partial
        // accumulator a single `combine` chain merges was created by the SAME
        // `CustomAggregate::init` factory call, with the SAME resolved
        // scalarvals (see the module docs' "Real merges" section and
        // `crate::agg_fn`'s "Merging structural state" section) — so, unlike
        // the old per-row positional-argument design, there is no cross-chunk
        // `p`-mismatch to detect here: concatenating the (still-unsorted)
        // value lists is always correct, since `finish` sorts the whole
        // multiset before computing a rank either way. A family mismatch
        // between the two partials (numeric vs duration) poisons, the same
        // as a within-accumulator mismatch in `step` above.
        let other = downcast_combine_partial::<Self>(other)?;
        self.state = match (
            mem::replace(&mut self.state, ValueSeries::Empty),
            other.state,
        ) {
            (ValueSeries::Poisoned, _) | (_, ValueSeries::Poisoned) => ValueSeries::Poisoned,
            (ValueSeries::Empty, s) | (s, ValueSeries::Empty) => s,
            (ValueSeries::Ok(mut values), ValueSeries::Ok(other_values)) => {
                if same_value_family(&values[0], &other_values[0]) {
                    values.extend(other_values);
                    ValueSeries::Ok(values)
                } else {
                    ValueSeries::Poisoned
                }
            }
        };
        Ok(())
    }

    /// See [`AggregateAccumulator::into_any`]'s trait docs — every implementor's
    /// body is this same one line.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let Self { p, state } = *self;
        let (Some(p), ValueSeries::Ok(mut values)) = (p, state) else {
            return Ok(None);
        };
        // See `MedianAccumulator::finish`'s identical comment on the
        // `value_cmp`-sourced, `.unwrap_or(Ordering::Equal)` sort policy.
        values.sort_by(|a, b| value_cmp(a, b).unwrap_or(Ordering::Equal));
        Ok(percentile_of(&values, &p).as_ref().map(xsd_value_to_term))
    }
}

struct PercentileAggregate;

impl CustomAggregate for PercentileAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Associative
    }
    fn state_bound(&self) -> u64 {
        VALUE_PROPORTIONAL_STATE_BOUND
    }
    fn scalarvals(&self) -> &[ScalarvalSpec] {
        const SPEC: [ScalarvalSpec; 1] = [ScalarvalSpec::new(PERCENTILE_P, ScalarvalKind::Numeric)];
        &SPEC
    }
    fn init(&self, scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(PercentileAccumulator {
            p: numeric_scalarval(scalarvals, PERCENTILE_P),
            state: ValueSeries::Empty,
        })
    }
}

// ---------------------------------------------------------------------------
// STDDEV / STDDEV_POP / VARIANCE / VAR_POP
// ---------------------------------------------------------------------------
//
// Implemented as a running `(n, sum, sum-of-squares)` fold — not Welford's
// incremental `(n, mean, M2)` recurrence — and the variance is recovered at
// `finish` via the identity `Var = (Σx² − (Σx)²/n) / denom`. This is a
// deliberate refinement of the plan's "Welford-style" starting point: Welford
// exists to solve TWO problems, and this representation gets BOTH without
// Welford's cost. First, avoiding catastrophic cancellation under
// FLOATING-POINT re-association — irrelevant to this crate's EXACT
// decimal/integer tower, where `Σx²` and `(Σx)²` are exact integers/decimals
// with no cancellation error to avoid. Second, being incrementally mergeable
// across parallel partial folds — needed, since this family IS
// `Volatility::Stable` and folds across `crate::parallel::par_chunk_reduce_init`
// chunks (see the module docs' "Real merges" section) — but `(n, Σx, Σx²)`
// merges through PLAIN componentwise addition (`MomentsAccumulator::combine`),
// simpler and cheaper than Welford's own pairwise-merge formula, and exactly
// as exact as the sequential fold. What Welford's per-row division WOULD cost
// here, with no benefit over the sum/sum-of-squares form's simpler merge, is
// precision: `mean = mean + delta/n` rounds once PER ROW at this crate's
// 18-fractional-digit decimal-division ceiling
// (`purrdf_xsd::numeric::MAX_DECIMAL_SCALE`), so an all-integer group's
// population variance can drift off its exact integer answer by a few units
// in the 18th digit. The sum/sum-of-squares form divides exactly ONCE (twice,
// for the final `Σx²ᵢ − (Σx)²/n` and once more for `/ denom`), so an
// all-integer or all-decimal group's variance stays EXACTLY the textbook
// answer regardless of how many chunks it was folded across — see this
// module's `var_pop_matches_the_known_dataset` test, which pins the exact
// `"4"` this form produces, and
// `stat_agg_moments_chunked_fold_forced_parallel_and_sequential_agree` in
// `crate::modifier`'s tests, which pins that a chunked fold reproduces it
// exactly.

#[derive(Clone, Copy)]
enum MomentsKind {
    Stddev,
    StddevPop,
    Variance,
    VarPop,
}

enum MomentsState {
    Empty,
    Ok {
        n: u64,
        sum: XsdValue,
        sumsq: XsdValue,
    },
    Poisoned,
}

/// One running-moments update: fold `x` into `(n, Σx, Σx²)`, returning the
/// next triple or `None` on arithmetic failure (poison).
fn moments_step(
    n: u64,
    sum: &XsdValue,
    sumsq: &XsdValue,
    x: &XsdValue,
) -> Option<(u64, XsdValue, XsdValue)> {
    let n1 = n.checked_add(1)?;
    let new_sum = numeric_add(sum, x).ok()?;
    let xsq = numeric_mul(x, x).ok()?;
    let new_sumsq = numeric_add(sumsq, &xsq).ok()?;
    Some((n1, new_sum, new_sumsq))
}

struct MomentsAccumulator {
    kind: MomentsKind,
    state: MomentsState,
}

impl AggregateAccumulator for MomentsAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if matches!(self.state, MomentsState::Poisoned) {
            return Ok(());
        }
        let Some(x) = args.first().and_then(xsd_of).filter(is_numeric_xsd) else {
            self.state = MomentsState::Poisoned;
            return Ok(());
        };
        let next = match &self.state {
            MomentsState::Empty => {
                let zero = numeric_sub(&x, &x).ok();
                zero.and_then(|zero| moments_step(0, &zero, &zero, &x))
            }
            MomentsState::Ok { n, sum, sumsq } => moments_step(*n, sum, sumsq, &x),
            MomentsState::Poisoned => None,
        };
        let Some((n, sum, sumsq)) = next else {
            self.state = MomentsState::Poisoned;
            return Ok(());
        };
        self.state = MomentsState::Ok { n, sum, sumsq };
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        // Componentwise moment merge: `(n, Σx, Σx²) + (n', Σx', Σx'²)` —
        // exact, no precision loss, for the same reason the running fold
        // itself is exact (see this family's own doc comment above). `other`'s
        // `kind` is discarded: it is the SAME variant as `self.kind` by
        // construction (one `MomentsAggregate::init` per accumulator).
        let other = downcast_combine_partial::<Self>(other)?;
        self.state = match (
            mem::replace(&mut self.state, MomentsState::Empty),
            other.state,
        ) {
            (MomentsState::Poisoned, _) | (_, MomentsState::Poisoned) => MomentsState::Poisoned,
            (MomentsState::Empty, s) | (s, MomentsState::Empty) => s,
            (
                MomentsState::Ok { n, sum, sumsq },
                MomentsState::Ok {
                    n: other_n,
                    sum: other_sum,
                    sumsq: other_sumsq,
                },
            ) => {
                let merged = n.checked_add(other_n).and_then(|n| {
                    let sum = numeric_add(&sum, &other_sum).ok()?;
                    let sumsq = numeric_add(&sumsq, &other_sumsq).ok()?;
                    Some((n, sum, sumsq))
                });
                match merged {
                    Some((n, sum, sumsq)) => MomentsState::Ok { n, sum, sumsq },
                    None => MomentsState::Poisoned,
                }
            }
        };
        Ok(())
    }

    /// See [`AggregateAccumulator::into_any`]'s trait docs — every implementor's
    /// body is this same one line.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let Self { kind, state } = *self;
        let MomentsState::Ok { n, sum, sumsq } = state else {
            return Ok(None);
        };
        let population = matches!(kind, MomentsKind::StddevPop | MomentsKind::VarPop);
        let denom = if population {
            Some(n)
        } else {
            n.checked_sub(1).filter(|&d| d > 0)
        };
        let Some(denom) = denom else {
            return Ok(None);
        };
        let n_val = XsdValue::Integer {
            value: i128::from(n),
            datatype: XsdDatatype::Integer,
        };
        let denom_val = XsdValue::Integer {
            value: i128::from(denom),
            datatype: XsdDatatype::Integer,
        };
        let Ok(sum_sq) = numeric_mul(&sum, &sum) else {
            return Ok(None);
        };
        let Ok(mean_correction) = numeric_div(&sum_sq, &n_val) else {
            return Ok(None);
        };
        let Ok(numerator) = numeric_sub(&sumsq, &mean_correction) else {
            return Ok(None);
        };
        let Ok(variance) = numeric_div(&numerator, &denom_val) else {
            return Ok(None);
        };
        match kind {
            MomentsKind::Variance | MomentsKind::VarPop => Ok(Some(xsd_value_to_term(&variance))),
            MomentsKind::Stddev | MomentsKind::StddevPop => {
                let Some(v) = to_f64(&variance) else {
                    return Ok(None);
                };
                let root = v.max(0.0).sqrt();
                Ok(Some(TermValue::typed_literal(
                    XsdValue::Double(root).canonical_lexical(),
                    XSD_DOUBLE,
                )))
            }
        }
    }
}

struct MomentsAggregate {
    kind: MomentsKind,
}

impl CustomAggregate for MomentsAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        MOMENTS_STATE_BOUND
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(MomentsAccumulator {
            kind: self.kind,
            state: MomentsState::Empty,
        })
    }
}

// ---------------------------------------------------------------------------
// MODE
// ---------------------------------------------------------------------------

struct ModeAccumulator {
    values: Vec<TermValue>,
}

impl AggregateAccumulator for ModeAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(v) = args.first() {
            self.values.push(v.clone());
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        // A count-map merge specialized to this accumulator's own
        // representation: `finish` recovers each value's count via a sort +
        // run-length scan over the WHOLE multiset, so appending the two
        // partials' raw value lists is exactly "sum the counts per value" —
        // no separate map is needed to get that effect.
        let mut other = downcast_combine_partial::<Self>(other)?;
        self.values.append(&mut other.values);
        Ok(())
    }

    /// See [`AggregateAccumulator::into_any`]'s trait docs — every implementor's
    /// body is this same one line.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let Self { mut values } = *self;
        if values.is_empty() {
            return Ok(None);
        }
        // Natural structural `Ord` (see `purrdf_core::TermValue`'s hand-written
        // impl) — used only to group exact-identity duplicates via a run-length
        // scan; the WINNER among equal-count runs is picked below by the SPARQL
        // value order, per the module docs.
        values.sort();
        let mut best: Option<(TermValue, u64)> = None;
        let mut i = 0;
        while i < values.len() {
            let mut j = i + 1;
            while j < values.len() && values[j] == values[i] {
                j += 1;
            }
            let count = u64::try_from(j - i).unwrap_or(u64::MAX);
            let better = match &best {
                None => true,
                Some((best_value, best_count)) => {
                    count > *best_count
                        || (count == *best_count
                            && match term_value_order(&values[i], best_value) {
                                Ordering::Less => true,
                                Ordering::Greater => false,
                                Ordering::Equal => values[i] < *best_value,
                            })
                }
            };
            if better {
                best = Some((values[i].clone(), count));
            }
            i = j;
        }
        Ok(best.map(|(value, _)| value))
    }
}

struct ModeAggregate;

impl CustomAggregate for ModeAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Associative
    }
    fn state_bound(&self) -> u64 {
        VALUE_PROPORTIONAL_STATE_BOUND
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(ModeAccumulator { values: Vec::new() })
    }
}

// ---------------------------------------------------------------------------
// FIRST / LAST
// ---------------------------------------------------------------------------

struct FirstAccumulator {
    value: Option<TermValue>,
}

impl AggregateAccumulator for FirstAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if self.value.is_none() {
            self.value = args.first().cloned();
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        // `self` is the earlier chunk: if it already saw a row, its value IS the
        // group's first value regardless of what a later chunk holds. Only an
        // empty earlier chunk defers to the later one.
        //
        // This relies on [`AggregateAccumulator::combine`]'s documented contract
        // ("`self` holds the earlier ... partial fold and `other` the later
        // one"), which `crate::parallel::par_chunk_reduce_init` upholds by
        // construction: it reduces `items.par_chunks(..)`'s per-chunk results
        // with a strictly sequential `for chunk_result in iter { combine(&mut
        // acc, chunk_result?)?; }` in CHUNK-INDEX order — never a tree/pairwise
        // reduction, and never reordered by which worker finishes first — so
        // `acc` (this `self`) is always chunk `i` and `chunk_result` (`other`)
        // is always chunk `i+1`, regardless of thread count or scheduling. A
        // flip of that order, or a switch to a non-sequential reduction shape,
        // would silently return the WRONG row's value with no local symptom —
        // see the module docs' "FIRST/LAST" section — which is why
        // `crate::modifier`'s `stat_agg_first_chunked_fold_forced_parallel_and_sequential_agree`
        // pins the exact answer (not just sequential/parallel agreement) over a
        // many-chunk forced-parallel fold.
        if self.value.is_none()
            && let Some(v) = other.finish()?
        {
            self.value = Some(v);
        }
        Ok(())
    }

    /// Unused (this accumulator merges through `finish()`, which is already
    /// sufficient state) — see [`AggregateAccumulator::into_any`]'s trait docs
    /// for why every implementor still supplies the one-line body.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.value)
    }
}

struct FirstAggregate;

impl CustomAggregate for FirstAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::OrderDependent
    }
    fn state_bound(&self) -> u64 {
        SCALAR_STATE_BOUND
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(FirstAccumulator { value: None })
    }
}

struct LastAccumulator {
    value: Option<TermValue>,
}

impl AggregateAccumulator for LastAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        self.value = args.first().cloned();
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        // `other` is the later chunk: whatever it holds is later in row order
        // than anything `self` holds, so it always wins when present.
        //
        // Same reliance as [`FirstAccumulator::combine`] on
        // [`AggregateAccumulator::combine`]'s documented earlier-`self`/later-
        // `other` contract, upheld by `crate::parallel::par_chunk_reduce_init`'s
        // fixed, strictly sequential chunk-index-order reduce (see that
        // `combine`'s comment for the exact mechanism) — pinned by
        // `crate::modifier`'s
        // `stat_agg_last_chunked_fold_forced_parallel_and_sequential_agree`,
        // which would fail on the group's FIRST value instead of its last if
        // that order ever flipped.
        if let Some(v) = other.finish()? {
            self.value = Some(v);
        }
        Ok(())
    }

    /// Unused (this accumulator merges through `finish()`, which is already
    /// sufficient state) — see [`AggregateAccumulator::into_any`]'s trait docs
    /// for why every implementor still supplies the one-line body.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.value)
    }
}

struct LastAggregate;

impl CustomAggregate for LastAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::OrderDependent
    }
    fn state_bound(&self) -> u64 {
        SCALAR_STATE_BOUND
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(LastAccumulator { value: None })
    }
}

// ---------------------------------------------------------------------------
// TOPK
// ---------------------------------------------------------------------------

enum TopKState {
    /// `k` resolved to a valid positive integer at `init` time (see
    /// `TopKAggregate::init`); `values` is the accumulator's live bounded
    /// top-`k` set, empty until the first row is folded. An empty `values` at
    /// `finish` — no row ever folded — is exactly the "empty group" case: see
    /// `finish`'s own `is_empty` check.
    Valid {
        k: usize,
        values: Vec<TermValue>,
    },
    Poisoned,
}

/// Insert `value` into the bounded top-`k` set, evicting the current smallest
/// (under [`term_value_order`], natural [`TermValue`] `Ord` as final tie-break)
/// once the set exceeds `k` — keeps the accumulator's live state at `O(k)`
/// elements at all times, never `O(group size)`.
fn insert_bounded(values: &mut Vec<TermValue>, k: usize, value: TermValue) {
    if k == 0 {
        return;
    }
    values.push(value);
    if values.len() > k {
        let min_idx = (0..values.len())
            .min_by(|&a, &b| {
                term_value_order(&values[a], &values[b]).then_with(|| values[a].cmp(&values[b]))
            })
            .expect("values is non-empty: just pushed one");
        values.remove(min_idx);
    }
}

struct TopKAccumulator {
    state: TopKState,
}

impl AggregateAccumulator for TopKAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        let TopKState::Valid { k, values } = &mut self.state else {
            return Ok(());
        };
        let Some(value) = args.first() else {
            self.state = TopKState::Poisoned;
            return Ok(());
        };
        insert_bounded(values, *k, value.clone());
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        // A bounded-structure merge: insert every one of `other`'s retained
        // values into `self`'s top-`k` set via the SAME `insert_bounded`
        // primitive `step` uses, truncating back down to `k` as it goes — the
        // merged state never exceeds `O(k)` elements, matching `step`'s own
        // bound. `k` is identical on both sides BY CONSTRUCTION (see
        // `PercentileAccumulator::combine`'s identical note on `p`), so there
        // is no cross-chunk `k`-mismatch left to detect, unlike the old
        // per-row positional-argument design.
        let other = downcast_combine_partial::<Self>(other)?;
        self.state = match (
            mem::replace(&mut self.state, TopKState::Poisoned),
            other.state,
        ) {
            (TopKState::Poisoned, _) | (_, TopKState::Poisoned) => TopKState::Poisoned,
            (
                TopKState::Valid { k, mut values },
                TopKState::Valid {
                    values: other_values,
                    ..
                },
            ) => {
                for value in other_values {
                    insert_bounded(&mut values, k, value);
                }
                TopKState::Valid { k, values }
            }
        };
        Ok(())
    }

    /// See [`AggregateAccumulator::into_any`]'s trait docs — every implementor's
    /// body is this same one line.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let Self { state } = *self;
        let TopKState::Valid { values, .. } = state else {
            return Ok(None);
        };
        if values.is_empty() {
            // No row was ever folded: the empty group answers unbound, like
            // every other member here.
            return Ok(None);
        }
        let mut sorted = values;
        // Descending: largest first — reverse-argument `term_value_order`/`Ord`.
        sorted.sort_by(|a, b| term_value_order(b, a).then_with(|| b.cmp(a)));
        let mut joined = String::new();
        for (i, v) in sorted.iter().enumerate() {
            let Some(lex) = lexical_of(v) else {
                // A blank node or a triple term has no lexical form — poison,
                // exactly as GROUP_CONCAT poisons on the same case.
                return Ok(None);
            };
            if i > 0 {
                joined.push(' ');
            }
            joined.push_str(&lex);
        }
        Ok(Some(TermValue::typed_literal(joined, XSD_STRING)))
    }
}

struct TopKAggregate;

impl CustomAggregate for TopKAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Associative
    }
    fn state_bound(&self) -> u64 {
        TOPK_STATE_BOUND
    }
    fn scalarvals(&self) -> &[ScalarvalSpec] {
        const SPEC: [ScalarvalSpec; 1] = [ScalarvalSpec::new(TOPK_K, ScalarvalKind::Numeric)];
        &SPEC
    }
    fn init(&self, scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        // `K` must be a positive `xsd:integer`, constant across the group — a
        // non-integer or non-positive `K` poisons the fold to unbound (never a
        // hard error), the same "poison, don't abort" discipline every other
        // numeric member here uses.
        let k = numeric_scalarval(scalarvals, TOPK_K).and_then(|v| match v {
            XsdValue::Integer { value, .. } if value > 0 => usize::try_from(value).ok(),
            _ => None,
        });
        Box::new(TopKAccumulator {
            state: match k {
                Some(k) => TopKState::Valid {
                    k,
                    values: Vec::new(),
                },
                None => TopKState::Poisoned,
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl AggregateRegistry {
    /// Register the crate's first-party statistical aggregate set —
    /// `MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`,
    /// `MODE`, `FIRST`, `LAST`, `TOPK` — under `namespace`, so each becomes
    /// reachable as `AGG(<{NAMESPACE}LOCAL-NAME>, args…)`.
    ///
    /// `namespace` is caller configuration, never a purrdf-owned vocabulary
    /// (see [`crate::stat_agg`]'s module docs) — there is no default: a host
    /// that never calls this gets none of these ten IRIs registered, exactly
    /// as a host that never configures
    /// [`purrdf_sparql_algebra::ParserOptions::extension_fn_namespaces`] gets
    /// none of [`purrdf_sparql_algebra::PurrdfFn`]'s scalar functions. Typically
    /// called once, right after [`AggregateRegistry::new`], before registering
    /// any host-specific aggregate of the caller's own.
    ///
    /// # Panics
    ///
    /// Panics if any of the ten IRIs (`namespace` concatenated with a member's
    /// local name) is already registered — see [`AggregateRegistry::register`]'s
    /// docs for why a registered aggregate may not be silently shadowed.
    pub fn register_statistical_aggregates(&mut self, namespace: &str) {
        self.register(format!("{namespace}{MEDIAN}"), Arc::new(MedianAggregate));
        self.register(
            format!("{namespace}{PERCENTILE}"),
            Arc::new(PercentileAggregate),
        );
        self.register(
            format!("{namespace}{STDDEV}"),
            Arc::new(MomentsAggregate {
                kind: MomentsKind::Stddev,
            }),
        );
        self.register(
            format!("{namespace}{STDDEV_POP}"),
            Arc::new(MomentsAggregate {
                kind: MomentsKind::StddevPop,
            }),
        );
        self.register(
            format!("{namespace}{VARIANCE}"),
            Arc::new(MomentsAggregate {
                kind: MomentsKind::Variance,
            }),
        );
        self.register(
            format!("{namespace}{VAR_POP}"),
            Arc::new(MomentsAggregate {
                kind: MomentsKind::VarPop,
            }),
        );
        self.register(format!("{namespace}{MODE}"), Arc::new(ModeAggregate));
        self.register(format!("{namespace}{FIRST}"), Arc::new(FirstAggregate));
        self.register(format!("{namespace}{LAST}"), Arc::new(LastAggregate));
        self.register(format!("{namespace}{TOPK}"), Arc::new(TopKAggregate));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = "http://example.org/agg#";

    fn iri(local: &str) -> String {
        format!("{NS}{local}")
    }

    fn registry() -> AggregateRegistry {
        let mut registry = AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);
        registry
    }

    fn int(n: i64) -> TermValue {
        TermValue::typed_literal(n.to_string(), "http://www.w3.org/2001/XMLSchema#integer")
    }

    fn dec(s: &str) -> TermValue {
        TermValue::typed_literal(s, "http://www.w3.org/2001/XMLSchema#decimal")
    }

    fn ym_dur(s: &str) -> TermValue {
        TermValue::typed_literal(s, "http://www.w3.org/2001/XMLSchema#yearMonthDuration")
    }

    fn dt_dur(s: &str) -> TermValue {
        TermValue::typed_literal(s, "http://www.w3.org/2001/XMLSchema#dayTimeDuration")
    }

    fn lex(t: &TermValue) -> String {
        match t {
            TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
            other => panic!("expected a literal, got {other:?}"),
        }
    }

    fn fold(local: &str, rows: &[Vec<TermValue>]) -> Option<TermValue> {
        fold_with(local, &[], rows)
    }

    /// [`fold`]'s twin for a member that reads a named scalarval at `init`
    /// time (`PERCENTILE`'s `P`, `TOPK`'s `K`) rather than a positional
    /// argument.
    fn fold_with(
        local: &str,
        scalarvals: &[(String, TermValue)],
        rows: &[Vec<TermValue>],
    ) -> Option<TermValue> {
        let registry = registry();
        let agg = registry.resolve(&iri(local)).expect("registered");
        let mut acc = agg.init(scalarvals);
        for row in rows {
            acc.step(row).expect("step");
        }
        acc.finish().expect("finish")
    }

    /// A `[(String, TermValue)]` scalarval slice with one numeric entry —
    /// the shape [`CustomAggregate::init`] receives after prepare-time
    /// resolution, built directly here (rather than through the parser) since
    /// these are unit tests of the accumulator, not the surface syntax.
    fn scalarval(name: &str, value: TermValue) -> Vec<(String, TermValue)> {
        vec![(name.to_owned(), value)]
    }

    // ---- registration / closed-set property --------------------------------

    #[test]
    fn registers_exactly_the_ten_members() {
        let registry = registry();
        assert_eq!(registry.len(), 10);
        for local in [
            MEDIAN, PERCENTILE, STDDEV, STDDEV_POP, VARIANCE, VAR_POP, MODE, FIRST, LAST, TOPK,
        ] {
            assert!(
                registry.resolve(&iri(local)).is_some(),
                "{local} must be registered"
            );
        }
    }

    #[test]
    fn a_name_outside_the_closed_set_is_unregistered() {
        let registry = registry();
        assert!(registry.resolve(&iri("NOT_A_MEMBER")).is_none());
        assert!(registry.resolve("http://example.org/agg#TOPK_V2").is_none());
    }

    #[test]
    #[should_panic(expected = "already registered as a custom aggregate")]
    fn registering_twice_under_the_same_namespace_panics() {
        let mut registry = AggregateRegistry::new();
        registry.register_statistical_aggregates(NS);
        registry.register_statistical_aggregates(NS);
    }

    // ---- MEDIAN --------------------------------------------------------------

    #[test]
    fn median_odd_group() {
        let v = fold(MEDIAN, &[vec![int(1)], vec![int(3)], vec![int(2)]]).expect("bound");
        assert_eq!(lex(&v), "2");
    }

    #[test]
    fn median_even_group_is_the_mean_of_the_two_middle_values() {
        let v = fold(
            MEDIAN,
            &[vec![int(1)], vec![int(2)], vec![int(3)], vec![int(4)]],
        )
        .expect("bound");
        assert_eq!(lex(&v), "2.5");
    }

    #[test]
    fn median_of_empty_group_is_unbound() {
        assert_eq!(fold(MEDIAN, &[]), None);
    }

    #[test]
    fn median_poisons_on_non_numeric_input() {
        let s = TermValue::simple_literal("oops");
        assert_eq!(fold(MEDIAN, &[vec![int(1)], vec![s]]), None);
    }

    /// `MEDIAN` over a pure `yearMonthDuration` group (see the module docs'
    /// "The `xsd:duration` extension" section) — odd count, so the median is
    /// one member of the group, not an interpolated value.
    #[test]
    fn median_over_pure_year_month_duration_group() {
        let rows = [ym_dur("P1Y"), ym_dur("P3Y"), ym_dur("P2Y")]
            .map(|t| vec![t])
            .to_vec();
        let v = fold(MEDIAN, &rows).expect("bound");
        assert_eq!(lex(&v), "P2Y");
    }

    /// `MEDIAN` over a pure `dayTimeDuration` group, even count: the midpoint
    /// arithmetic (`(a + b) / 2`, via `value_add`/`value_mul` — see the module
    /// docs) lands EXACTLY on a whole-day value, so the result is pinned
    /// without depending on how a fractional day canonicalizes.
    #[test]
    fn median_over_pure_day_time_duration_even_group_is_the_midpoint() {
        let rows = [dt_dur("P0D"), dt_dur("P2D"), dt_dur("P4D"), dt_dur("P6D")]
            .map(|t| vec![t])
            .to_vec();
        let v = fold(MEDIAN, &rows).expect("bound");
        assert_eq!(lex(&v), "P3D");
    }

    /// A group mixing a plain numeric value and a duration POISONS — the same
    /// discipline `crate::modifier`'s `SUM`/`AVG` duration extension already
    /// uses for the identical mixed case (see the module docs).
    #[test]
    fn median_over_mixed_numeric_and_duration_is_unbound() {
        assert_eq!(
            fold(MEDIAN, &[vec![int(1)], vec![dt_dur("P1D")]]),
            None,
            "numeric-then-duration must poison"
        );
        assert_eq!(
            fold(MEDIAN, &[vec![dt_dur("P1D")], vec![int(1)]]),
            None,
            "duration-then-numeric must poison"
        );
    }

    // ---- PERCENTILE ------------------------------------------------------------

    #[test]
    fn percentile_zero_is_the_minimum_and_one_is_the_maximum() {
        let rows = vec![vec![int(1)], vec![int(2)], vec![int(3)]];
        let p0 = fold_with(PERCENTILE, &scalarval(PERCENTILE_P, dec("0")), &rows).expect("bound");
        assert_eq!(lex(&p0), "1");
        let p1 = fold_with(PERCENTILE, &scalarval(PERCENTILE_P, dec("1")), &rows).expect("bound");
        assert_eq!(lex(&p1), "3");
    }

    #[test]
    fn percentile_out_of_range_poisons() {
        let rows = vec![vec![int(1)], vec![int(2)]];
        assert_eq!(
            fold_with(PERCENTILE, &scalarval(PERCENTILE_P, dec("1.5")), &rows),
            None
        );
    }

    /// `P` is a named scalarval — a single value for the WHOLE aggregation —
    /// so there is no longer a "P differs per row" case to poison on (that
    /// was only representable under the old two-positional-argument shape).
    /// A missing `P` (an empty scalarval slice, the shape prepare-time
    /// validation would have refused before evaluation ever reached this
    /// accumulator) still poisons, exactly as `TOPK`'s missing `K` does — the
    /// defense-in-depth path `numeric_scalarval`'s docs describe.
    #[test]
    fn percentile_missing_p_poisons() {
        let rows = vec![vec![int(1)], vec![int(2)]];
        assert_eq!(fold_with(PERCENTILE, &[], &rows), None);
    }

    /// `PERCENTILE` over a pure `dayTimeDuration` group, at a `p` chosen so
    /// the interpolated rank is NOT a whole number (real interpolation, not a
    /// data point): `n = 2`, `rank = 0.37 * 1 = 0.37`, interpolating 37% of
    /// the way from `P0D` to `P100D` — `P37D` exactly (`100 * 0.37 = 37`, no
    /// fractional day, so the pin does not depend on sub-day canonicalization).
    #[test]
    fn percentile_over_pure_day_time_duration_needs_real_interpolation() {
        let rows = vec![vec![dt_dur("P0D")], vec![dt_dur("P100D")]];
        let v = fold_with(PERCENTILE, &scalarval(PERCENTILE_P, dec("0.37")), &rows).expect("bound");
        assert_eq!(lex(&v), "P37D");
    }

    /// `PERCENTILE` over a pure `yearMonthDuration` group, same non-data-point
    /// interpolation shape as the `dayTimeDuration` test above: `37%` of the
    /// way from `P0M` to `P100M` is `P37M` (`3` years `1` month), exact.
    #[test]
    fn percentile_over_pure_year_month_duration_needs_real_interpolation() {
        let rows = vec![vec![ym_dur("P0M")], vec![ym_dur("P100M")]];
        let v = fold_with(PERCENTILE, &scalarval(PERCENTILE_P, dec("0.37")), &rows).expect("bound");
        assert_eq!(lex(&v), "P3Y1M");
    }

    /// A mixed-SUBTYPE duration group (`yearMonthDuration` beside
    /// `dayTimeDuration`) is NOT mixed-FAMILY — it does not poison (see the
    /// module docs' "The `xsd:duration` extension" section). `P0M` and `P0D`
    /// are both the ZERO duration (comparable across subtypes — see
    /// `purrdf_xsd::ops`'s "Zero is one value in both subtypes' value spaces"
    /// test), so `p = 1` (the maximum) deterministically lands on `P100D`.
    #[test]
    fn percentile_over_mixed_subtype_duration_group_does_not_poison() {
        let rows = vec![vec![ym_dur("P0M")], vec![dt_dur("P100D")]];
        let v = fold_with(PERCENTILE, &scalarval(PERCENTILE_P, dec("1")), &rows).expect("bound");
        assert_eq!(lex(&v), "P100D");
    }

    /// A group mixing a plain numeric value and a duration POISONS — mirrors
    /// `median_over_mixed_numeric_and_duration_is_unbound`.
    #[test]
    fn percentile_over_mixed_numeric_and_duration_is_unbound() {
        let scalars = scalarval(PERCENTILE_P, dec("0.5"));
        assert_eq!(
            fold_with(PERCENTILE, &scalars, &[vec![int(1)], vec![dt_dur("P1D")]]),
            None,
            "numeric-then-duration must poison"
        );
        assert_eq!(
            fold_with(PERCENTILE, &scalars, &[vec![dt_dur("P1D")], vec![int(1)]]),
            None,
            "duration-then-numeric must poison"
        );
    }

    // ---- STDDEV / STDDEV_POP / VARIANCE / VAR_POP -----------------------------

    // Known dataset: {2, 4, 4, 4, 5, 5, 7, 9} — population variance 4, population
    // stddev 2, sample variance 32/7, sample stddev sqrt(32/7).
    fn moments_rows() -> Vec<Vec<TermValue>> {
        [2, 4, 4, 4, 5, 5, 7, 9]
            .into_iter()
            .map(|n| vec![int(n)])
            .collect()
    }

    #[test]
    fn var_pop_matches_the_known_dataset() {
        let v = fold(VAR_POP, &moments_rows()).expect("bound");
        assert_eq!(lex(&v), "4");
    }

    #[test]
    fn stddev_pop_matches_the_known_dataset() {
        let v = fold(STDDEV_POP, &moments_rows()).expect("bound");
        assert_eq!(lex(&v), "2.0E0");
    }

    #[test]
    fn variance_and_stddev_relationship_holds() {
        let variance = fold(VARIANCE, &moments_rows()).expect("bound");
        let stddev = fold(STDDEV, &moments_rows()).expect("bound");
        let variance_f64: f64 = lex(&variance).parse().expect("decimal");
        let stddev_f64: f64 = lex(&stddev).parse().expect("double");
        assert!(
            stddev_f64.mul_add(stddev_f64, -variance_f64).abs() < 1e-9,
            "stddev^2 must equal variance: {stddev_f64}^2 != {variance_f64}"
        );
    }

    #[test]
    fn sample_variance_of_a_single_value_group_is_unbound() {
        assert_eq!(fold(VARIANCE, &[vec![int(5)]]), None);
        assert_eq!(fold(STDDEV, &[vec![int(5)]]), None);
    }

    #[test]
    fn population_variance_of_a_single_value_group_is_zero() {
        let v = fold(VAR_POP, &[vec![int(5)]]).expect("bound");
        assert_eq!(lex(&v), "0");
    }

    #[test]
    fn moments_over_empty_group_are_unbound() {
        for name in [STDDEV, STDDEV_POP, VARIANCE, VAR_POP] {
            assert_eq!(fold(name, &[]), None, "{name} over empty group");
        }
    }

    /// `STDDEV`/`STDDEV_POP`/`VARIANCE`/`VAR_POP` stay numeric-only (see the
    /// module docs' "Why `STDDEV`/.../`VAR_POP` stay numeric-only" section:
    /// `xsd:duration × xsd:duration` has no value space) — a duration input
    /// poisons the fold exactly like any other non-numeric value, unlike
    /// `MEDIAN`/`PERCENTILE`'s widened gate.
    #[test]
    fn moments_poison_on_duration_input() {
        for name in [STDDEV, STDDEV_POP, VARIANCE, VAR_POP] {
            assert_eq!(
                fold(name, &[vec![dt_dur("P1D")], vec![dt_dur("P2D")]]),
                None,
                "{name} must poison on a duration input"
            );
        }
    }

    // ---- MODE ------------------------------------------------------------------

    #[test]
    fn mode_picks_the_most_frequent_value() {
        let rows = [1, 2, 2, 3, 2]
            .into_iter()
            .map(|n| vec![int(n)])
            .collect::<Vec<_>>();
        let v = fold(MODE, &rows).expect("bound");
        assert_eq!(lex(&v), "2");
    }

    #[test]
    fn mode_tie_break_keeps_the_smallest_value() {
        let rows = vec![vec![int(5)], vec![int(1)], vec![int(5)], vec![int(1)]];
        let v = fold(MODE, &rows).expect("bound");
        assert_eq!(lex(&v), "1");
    }

    #[test]
    fn mode_works_over_iris() {
        let rows = vec![
            vec![TermValue::iri("http://ex/a")],
            vec![TermValue::iri("http://ex/b")],
            vec![TermValue::iri("http://ex/a")],
        ];
        let v = fold(MODE, &rows).expect("bound");
        assert_eq!(v, TermValue::iri("http://ex/a"));
    }

    #[test]
    fn mode_of_empty_group_is_unbound() {
        assert_eq!(fold(MODE, &[]), None);
    }

    /// `MODE` needed no code change to accept durations (see the module docs'
    /// "The `xsd:duration` extension" section: it folds over any [`TermValue`]
    /// by RDF term identity, never gated on `is_numeric_xsd`).
    #[test]
    fn mode_over_durations_picks_the_most_frequent_value() {
        let rows = [dt_dur("P1D"), dt_dur("P2D"), dt_dur("P2D")]
            .map(|t| vec![t])
            .to_vec();
        let v = fold(MODE, &rows).expect("bound");
        assert_eq!(lex(&v), "P2D");
    }

    // ---- FIRST / LAST ------------------------------------------------------------

    #[test]
    fn first_and_last_over_input_row_order() {
        let rows = vec![vec![int(10)], vec![int(20)], vec![int(30)]];
        assert_eq!(lex(&fold(FIRST, &rows).expect("bound")), "10");
        assert_eq!(lex(&fold(LAST, &rows).expect("bound")), "30");
    }

    #[test]
    fn first_and_last_of_empty_group_are_unbound() {
        assert_eq!(fold(FIRST, &[]), None);
        assert_eq!(fold(LAST, &[]), None);
    }

    /// `FIRST`/`LAST` needed no code change to accept durations — same
    /// reasoning as `mode_over_durations_picks_the_most_frequent_value`.
    #[test]
    fn first_and_last_over_durations_in_input_row_order() {
        let rows = [dt_dur("P1D"), dt_dur("P2D"), dt_dur("P3D")]
            .map(|t| vec![t])
            .to_vec();
        assert_eq!(lex(&fold(FIRST, &rows).expect("bound")), "P1D");
        assert_eq!(lex(&fold(LAST, &rows).expect("bound")), "P3D");
    }

    #[test]
    fn first_combine_keeps_the_earlier_chunks_value() {
        let registry = registry();
        let agg = registry.resolve(&iri(FIRST)).expect("registered");
        let mut a = agg.init(&[]);
        a.step(&[int(1)]).expect("step");
        let mut b = agg.init(&[]);
        b.step(&[int(2)]).expect("step");
        a.combine(b).expect("combine");
        assert_eq!(lex(&a.finish().expect("finish").expect("bound")), "1");
    }

    #[test]
    fn last_combine_keeps_the_later_chunks_value() {
        let registry = registry();
        let agg = registry.resolve(&iri(LAST)).expect("registered");
        let mut a = agg.init(&[]);
        a.step(&[int(1)]).expect("step");
        let mut b = agg.init(&[]);
        b.step(&[int(2)]).expect("step");
        a.combine(b).expect("combine");
        assert_eq!(lex(&a.finish().expect("finish").expect("bound")), "2");
    }

    // ---- TOPK --------------------------------------------------------------------

    #[test]
    fn topk_returns_the_largest_k_values_descending() {
        let rows = [3, 1, 4, 1, 5, 9, 2, 6]
            .into_iter()
            .map(|n| vec![int(n)])
            .collect::<Vec<_>>();
        let v = fold_with(TOPK, &scalarval(TOPK_K, int(3)), &rows).expect("bound");
        assert_eq!(lex(&v), "9 6 5");
    }

    #[test]
    fn topk_with_fewer_values_than_k_returns_all_of_them() {
        let rows = vec![vec![int(1)], vec![int(2)]];
        let v = fold_with(TOPK, &scalarval(TOPK_K, int(5)), &rows).expect("bound");
        assert_eq!(lex(&v), "2 1");
    }

    #[test]
    fn topk_non_positive_k_poisons() {
        let rows = vec![vec![int(1)]];
        assert_eq!(fold_with(TOPK, &scalarval(TOPK_K, int(0)), &rows), None);
        assert_eq!(fold_with(TOPK, &scalarval(TOPK_K, int(-1)), &rows), None);
    }

    #[test]
    fn topk_non_integer_k_poisons() {
        let rows = vec![vec![int(1)]];
        assert_eq!(fold_with(TOPK, &scalarval(TOPK_K, dec("2.5")), &rows), None);
    }

    /// `K` is a named scalarval, so there is no longer a "K differs per row"
    /// case to poison on — see `percentile_missing_p_poisons`'s identical note
    /// for `PERCENTILE`'s `P`. A missing `K` still poisons.
    #[test]
    fn topk_missing_k_poisons() {
        let rows = vec![vec![int(1)], vec![int(2)]];
        assert_eq!(fold_with(TOPK, &[], &rows), None);
    }

    #[test]
    fn topk_of_empty_group_is_unbound() {
        assert_eq!(fold_with(TOPK, &scalarval(TOPK_K, int(3)), &[]), None);
    }

    /// `TOPK` needed no code change to accept durations — its total order is
    /// `crate::modifier::term_value_order`, which already orders duration
    /// literals correctly (see the module docs' "The `xsd:duration`
    /// extension" section).
    #[test]
    fn topk_over_durations_returns_the_largest_k_descending() {
        let rows = [
            dt_dur("P3D"),
            dt_dur("P1D"),
            dt_dur("P4D"),
            dt_dur("P1D"),
            dt_dur("P5D"),
        ]
        .map(|t| vec![t])
        .to_vec();
        let v = fold_with(TOPK, &scalarval(TOPK_K, int(3)), &rows).expect("bound");
        assert_eq!(lex(&v), "P5D P4D P3D");
    }

    // ---- combine (parallel-reducer merge) coverage ----------------------------
    //
    // `MEDIAN`, `PERCENTILE`, `MODE`, and `TOPK` merge through `into_any`'s
    // type-erased downcast escape hatch (see the module docs' "Real merges via
    // `AggregateAccumulator::into_any`" section) rather than through `finish()` —
    // the module's own docs call this the subtle part of the design, and a defect
    // here produces a wrong value only under a parallel/chunked fold, never under
    // a sequential one. Each test below builds two partial accumulators by hand
    // (exactly what `crate::parallel::par_chunk_reduce_init` produces per chunk),
    // `combine`s them, and checks the merged answer against a direct,
    // single-accumulator fold over the concatenated dataset — the same oracle
    // `first_combine_keeps_the_earlier_chunks_value`/
    // `last_combine_keeps_the_later_chunks_value` already use for `FIRST`/`LAST`.

    #[test]
    fn median_combine_merges_two_partial_value_lists() {
        let registry = registry();
        let agg = registry.resolve(&iri(MEDIAN)).expect("registered");

        let mut a = agg.init(&[]);
        a.step(&[int(1)]).expect("step");
        a.step(&[int(3)]).expect("step");
        let mut b = agg.init(&[]);
        b.step(&[int(2)]).expect("step");
        b.step(&[int(4)]).expect("step");
        a.combine(b).expect("combine");
        let merged = a.finish().expect("finish").expect("bound");

        let direct = fold(
            MEDIAN,
            &[vec![int(1)], vec![int(3)], vec![int(2)], vec![int(4)]],
        )
        .expect("bound");
        assert_eq!(lex(&merged), lex(&direct));
        assert_eq!(lex(&merged), "2.5");
    }

    #[test]
    fn median_combine_poisons_if_either_side_is_poisoned() {
        let registry = registry();
        let agg = registry.resolve(&iri(MEDIAN)).expect("registered");

        let mut a = agg.init(&[]);
        a.step(&[int(1)]).expect("step");
        let mut poisoned = agg.init(&[]);
        poisoned
            .step(&[TermValue::simple_literal("oops")])
            .expect("step");

        a.combine(poisoned).expect("combine");
        assert_eq!(a.finish().expect("finish"), None);
    }

    /// `MEDIAN` over `xsd:duration` partials, merged through the SAME
    /// `into_any`-downcast real merge `median_combine_merges_two_partial_value_lists`
    /// exercises for numerics — this is `crate::stat_agg`'s share of the
    /// "forced-parallel agreement" coverage the task's test plan asks for; the
    /// dataset-level, real `crate::parallel::par_chunk_reduce_init` path is
    /// additionally pinned in `crate::modifier`'s
    /// `stat_agg_median_duration_chunked_fold_forced_parallel_and_sequential_agree`.
    #[test]
    fn median_combine_merges_two_partial_duration_value_lists() {
        let registry = registry();
        let agg = registry.resolve(&iri(MEDIAN)).expect("registered");

        let mut a = agg.init(&[]);
        a.step(&[dt_dur("P0D")]).expect("step");
        a.step(&[dt_dur("P4D")]).expect("step");
        let mut b = agg.init(&[]);
        b.step(&[dt_dur("P2D")]).expect("step");
        b.step(&[dt_dur("P6D")]).expect("step");
        a.combine(b).expect("combine");
        let merged = a.finish().expect("finish").expect("bound");

        let direct = fold(
            MEDIAN,
            &[
                vec![dt_dur("P0D")],
                vec![dt_dur("P4D")],
                vec![dt_dur("P2D")],
                vec![dt_dur("P6D")],
            ],
        )
        .expect("bound");
        assert_eq!(lex(&merged), lex(&direct));
        assert_eq!(lex(&merged), "P3D");
    }

    /// A family mismatch (numeric vs duration) between the two partials
    /// poisons the merge too, not just a within-accumulator mismatch (see
    /// `median_over_mixed_numeric_and_duration_is_unbound`).
    #[test]
    fn median_combine_poisons_on_a_family_mismatch_between_partials() {
        let registry = registry();
        let agg = registry.resolve(&iri(MEDIAN)).expect("registered");

        let mut a = agg.init(&[]);
        a.step(&[int(1)]).expect("step");
        let mut b = agg.init(&[]);
        b.step(&[dt_dur("P1D")]).expect("step");

        a.combine(b).expect("combine");
        assert_eq!(a.finish().expect("finish"), None);
    }

    #[test]
    fn percentile_combine_merges_two_partial_value_lists() {
        let registry = registry();
        let agg = registry.resolve(&iri(PERCENTILE)).expect("registered");
        let scalars = scalarval(PERCENTILE_P, dec("1"));

        let mut a = agg.init(&scalars);
        a.step(&[int(1)]).expect("step");
        let mut b = agg.init(&scalars);
        b.step(&[int(3)]).expect("step");
        b.step(&[int(2)]).expect("step");
        a.combine(b).expect("combine");
        let merged = a.finish().expect("finish").expect("bound");

        let direct = fold_with(
            PERCENTILE,
            &scalars,
            &[vec![int(1)], vec![int(3)], vec![int(2)]],
        )
        .expect("bound");
        assert_eq!(lex(&merged), lex(&direct));
        assert_eq!(lex(&merged), "3"); // p=1 is the maximum of {1, 2, 3}
    }

    #[test]
    fn percentile_combine_poisons_if_either_side_is_poisoned() {
        let registry = registry();
        let agg = registry.resolve(&iri(PERCENTILE)).expect("registered");

        let mut a = agg.init(&scalarval(PERCENTILE_P, dec("0.5")));
        a.step(&[int(1)]).expect("step");
        // A missing `P` poisons at `init` time (see `percentile_missing_p_poisons`).
        let mut poisoned = agg.init(&[]);
        poisoned.step(&[int(2)]).expect("step");

        a.combine(poisoned).expect("combine");
        assert_eq!(a.finish().expect("finish"), None);
    }

    #[test]
    fn mode_combine_merges_two_partial_value_multisets() {
        let registry = registry();
        let agg = registry.resolve(&iri(MODE)).expect("registered");

        // Neither partial has a majority for `2` alone (2-vs-1 in `a`, 1-vs-1 in
        // `b`): only merging the RAW multisets — not either side's `finish()`
        // winner — recovers that `2` occurs three times overall.
        let mut a = agg.init(&[]);
        a.step(&[int(1)]).expect("step");
        a.step(&[int(2)]).expect("step");
        a.step(&[int(2)]).expect("step");
        let mut b = agg.init(&[]);
        b.step(&[int(2)]).expect("step");
        b.step(&[int(3)]).expect("step");
        a.combine(b).expect("combine");
        let merged = a.finish().expect("finish").expect("bound");

        let direct = fold(
            MODE,
            &[
                vec![int(1)],
                vec![int(2)],
                vec![int(2)],
                vec![int(2)],
                vec![int(3)],
            ],
        )
        .expect("bound");
        assert_eq!(lex(&merged), lex(&direct));
        assert_eq!(lex(&merged), "2");
    }

    #[test]
    fn topk_combine_merges_two_partial_bounded_sets() {
        let registry = registry();
        let agg = registry.resolve(&iri(TOPK)).expect("registered");
        let scalars = scalarval(TOPK_K, int(3));

        let mut a = agg.init(&scalars);
        for v in [3, 1, 4] {
            a.step(&[int(v)]).expect("step");
        }
        let mut b = agg.init(&scalars);
        for v in [1, 5, 9, 2, 6] {
            b.step(&[int(v)]).expect("step");
        }
        a.combine(b).expect("combine");
        let merged = a.finish().expect("finish").expect("bound");

        // Must match a single-accumulator fold over the whole dataset — the merge
        // must recover the true top 3 across BOTH partials, not just keep
        // whichever side's own top-3 happened to be computed first.
        let rows = [3, 1, 4, 1, 5, 9, 2, 6]
            .into_iter()
            .map(|n| vec![int(n)])
            .collect::<Vec<_>>();
        let direct = fold_with(TOPK, &scalars, &rows).expect("bound");
        assert_eq!(lex(&merged), lex(&direct));
        assert_eq!(lex(&merged), "9 6 5");
    }

    #[test]
    fn topk_combine_poisons_if_either_side_is_poisoned() {
        let registry = registry();
        let agg = registry.resolve(&iri(TOPK)).expect("registered");

        let mut a = agg.init(&scalarval(TOPK_K, int(3)));
        a.step(&[int(1)]).expect("step");
        // A non-positive `K` poisons at `init` time (see `topk_non_positive_k_poisons`).
        let mut poisoned = agg.init(&scalarval(TOPK_K, int(0)));
        poisoned.step(&[int(2)]).expect("step");

        a.combine(poisoned).expect("combine");
        assert_eq!(a.finish().expect("finish"), None);
    }
}
