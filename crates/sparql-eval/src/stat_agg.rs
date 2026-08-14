// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A first-party, CLOSED set of statistical aggregates, delivered through the
//! [`crate::agg_fn`] custom-aggregate seam under a **caller-configured**
//! namespace — never a purrdf-owned vocabulary (see the crate's "mints no
//! vocabulary IRIs" contract). [`AggregateRegistry::register_statistical_aggregates`]
//! is the single entry point: a Rust host that already constructs an
//! [`AggregateRegistry`] (for [`crate::engine::QueryOptions::aggregates`]) calls it
//! once with a namespace IRI, and every member below becomes reachable from the
//! query text as `AGG(<namespace><LOCAL-NAME>, args…)` — a Python/WASM/C/CLI
//! binding that already exposes registry configuration gets the whole set for
//! free, through the string surface, with zero additional Rust callbacks.
//!
//! # The shipped set
//!
//! `MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
//! `FIRST`, `LAST`, `TOPK` — ten members, closed (no caller extension point; a
//! local name outside this set under the configured namespace is registered
//! nowhere, so `AGG(<ns>NOT_A_MEMBER, ?x)` is refused at prepare time exactly as
//! any other unregistered custom-aggregate IRI is).
//!
//! # Numeric discipline
//!
//! Every numeric member works over [`purrdf_xsd::XsdValue`]'s promotion tower
//! (`integer ⊂ decimal ⊂ float ⊂ double`), through the SAME `numeric_add`/
//! `numeric_sub`/`numeric_mul`/`numeric_div`/`numeric_floor` primitives
//! [`crate::modifier`]'s built-in `SUM`/`AVG` fold uses — a mixed integer/decimal
//! group promotes to decimal exactly the way `AVG(1, 0.5)` already does, and
//! division rounds at the same 18-fractional-digit ceiling `AVG` already accepts.
//! A non-numeric input to a numeric member **poisons** the fold to unbound —
//! `SUM`'s own discipline (see [`crate::modifier`]'s "Aggregate error handling"
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
//! # `PERCENTILE`'s two-argument form
//!
//! `AGG(<ns>PERCENTILE, ?x, ?p)` — `p` is evaluated per row, like every other
//! aggregate argument (the custom-aggregate surface takes positional
//! expression arguments only; there is no separate "parameter" channel), but
//! it must be numerically the SAME value on every row the group folds: a
//! group whose `p` differs across rows has no single percentile to report, so
//! it poisons to unbound exactly as a non-numeric value does. `p` outside
//! `[0, 1]` likewise poisons to unbound (never a hard error — the SAME
//! "poison, don't abort" discipline every other numeric member uses).
//!
//! # `MODE`
//!
//! Works over **any** term kind (it counts term occurrences by RDF term
//! identity — [`purrdf_core::TermValue`]'s own `Eq`, not "value equality":
//! `"5"^^xsd:integer` and `"05"^^xsd:integer` are numerically equal but are
//! counted as two DIFFERENT terms, exactly as `DISTINCT`'s own dedup treats
//! them). A tie among several terms with the same maximum count is broken by
//! the smallest term under the SAME total order `MIN`/`ORDER BY` use
//! ([`crate::modifier::term_value_order`]); a further tie (distinct terms
//! that compare value-equal under that order, e.g. `"5"`/`"05"`) falls back to
//! [`purrdf_core::TermValue`]'s own canonical structural order, so the choice
//! is fully deterministic regardless of scan order.
//!
//! # `FIRST`/`LAST`
//!
//! Work over any term kind, in **input row order** (the same row order
//! `GROUP_CONCAT`/`SAMPLE` fold over — see [`crate::modifier`]'s module docs):
//! `FIRST` is the earliest row's value, `LAST` the latest.
//!
//! # `TOPK`'s two-argument form and its "one term" contract
//!
//! `AGG(<ns>TOPK, ?x, ?k)` — `k` is a positive `xsd:integer`, constant across
//! the group (the SAME per-row-constant discipline `PERCENTILE`'s `p` uses; a
//! non-integer, non-positive, or inconsistent `k` poisons to unbound). Every
//! SPARQL aggregate's contract is to yield exactly ONE RDF term (this crate
//! has no CDT-list container type to hand back "the k values" as a list), so
//! `TOPK` answers the way `GROUP_CONCAT` already answers an inherently
//! multi-valued question: the top `k` values, in DESCENDING order under the
//! SAME total value order `MIN`/`MAX`/`MODE` use, with their **lexical forms**
//! joined by a single fixed space separator (mirroring
//! [`crate::modifier`]'s `GROUP_CONCAT`, whose default separator is likewise
//! `" "` per SPARQL §18.6.1.7). A configurable separator would need a THIRD
//! positional argument; two-argument `TOPK(value, k)` keeps the arity
//! contract as simple as `PERCENTILE`'s, so the separator is fixed by design,
//! not by oversight. A term with no lexical form (a blank node or a triple
//! term — the same case `GROUP_CONCAT` itself poisons on) poisons `TOPK` too.
//! Fewer than `k` distinct rows in the group → every value the group has.
//! Empty group → unbound, like every other member here.
//!
//! # Why these are `Volatility::Volatile` (all except `FIRST`/`LAST`)
//!
//! [`crate::agg_fn::AggregateAccumulator::combine`] takes `other:
//! Box<dyn AggregateAccumulator>` — a fully TYPE-ERASED accumulator whose only
//! observable surface is `combine`'s own caller calling `other.finish()` (the
//! pattern this crate's own fixtures use — see `crate::agg_fn`'s
//! `SumAccumulator` test and `crate::modifier`'s `ListCollector` test). That
//! pattern is sound EXACTLY when a fold's `finish()` output is *itself*
//! sufficient mergeable state — true for `SUM` (a running total IS the raw
//! state) and `GROUP_CONCAT`/list-collection (a joined string IS the raw
//! state, losslessly re-splittable). It is NOT true for a statistic whose
//! finished, single-term answer throws away the information a correct merge
//! needs: two partial standard deviations cannot be combined into the whole
//! group's standard deviation without each side's count and mean too; two
//! partial medians cannot be combined into the whole group's median at all
//! (median is not a composable reduction); two partial modes cannot be
//! combined without each side's per-value counts, which a single winning term
//! does not carry.
//!
//! Given that constraint, declaring `Volatility::Stable` for any of these
//! seven and implementing `combine` via `other.finish()` would either be
//! silently WRONG under within-group chunking (the exact failure the
//! `CustomAggregate::volatility` docs warn a misdeclaration causes) or would
//! require inventing a lossy re-serialization that corrupts the group's TRUE
//! final answer (since `finish()` has exactly one behaviour regardless of
//! whether the evaluator or this module's own `combine` is the caller — see
//! the long-form reasoning kept in this crate's development history for the
//! full argument). `Volatility::Volatile` is therefore the HONEST declaration
//! for `MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`,
//! `MODE`, and `TOPK`: it pins each to the single-accumulator sequential fold
//! this crate has always supported (see `crate::agg_fn`'s module docs), which
//! is what makes their answer exact and — trivially, since no chunking ever
//! happens — byte-identical between a "parallel" and a "sequential" run.
//! `combine` is still implemented for each (the trait requires it), as a
//! documented, panic-contained `unreachable!()`: the evaluator's chunked fold
//! never calls it for a `Volatile` aggregate, so firing it anyway is a defect
//! in the evaluator, not a state this module needs to handle.
//!
//! `FIRST`/`LAST` are the two members whose `finish()` output (the earliest
//! or latest value seen SO FAR) genuinely is sufficient state — an earlier
//! chunk's first value stays the group's first value no matter what a later
//! chunk saw, and symmetrically for `LAST` — so both stay
//! `Volatility::Stable` with a correct, real `combine`.
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
use std::sync::Arc;

use purrdf_core::TermValue;
use purrdf_xsd::numeric::{numeric_cmp, numeric_eq};
use purrdf_xsd::{
    XsdDatatype, XsdValue, numeric_add, numeric_div, numeric_floor, numeric_mul, numeric_sub,
};

use crate::agg_fn::{AggregateAccumulator, AggregateRegistry, AlgebraicClass, CustomAggregate};
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

/// The Welford moment accumulators' declared [`CustomAggregate::state_bound`]:
/// genuinely `O(1)` — see the module docs.
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

/// The panic message every `Volatile` member's unreachable `combine` shares —
/// see the module docs' "Why these are `Volatility::Volatile`" section.
fn volatile_combine_unreachable(local_name: &str) -> ! {
    unreachable!(
        "AGG <…>{local_name} declares Volatility::Volatile, so \
         crate::modifier::eval_custom_aggregate's chunked fold never invokes combine \
         for it (chunk_count is forced to 1) — see purrdf_sparql_eval::stat_agg's module \
         docs for why a real combine is not implementable through the erased \
         AggregateAccumulator seam for this member"
    )
}

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

/// The `p`-th percentile of an already value-order-sorted, non-empty numeric
/// series, under linear interpolation between the two closest ranks (see the
/// module docs). `None` (poison) when `p` is outside `[0, 1]` or any step of
/// the promoted-numeric-tower arithmetic fails.
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
    let diff = numeric_sub(&sorted[hi_idx], &sorted[lo_idx]).ok()?;
    let scaled = numeric_mul(&diff, &fraction).ok()?;
    numeric_add(&sorted[lo_idx], &scaled).ok()
}

// ---------------------------------------------------------------------------
// MEDIAN
// ---------------------------------------------------------------------------

enum NumericSeries {
    Empty,
    Ok(Vec<XsdValue>),
    Poisoned,
}

struct MedianAccumulator {
    state: NumericSeries,
}

impl AggregateAccumulator for MedianAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if matches!(self.state, NumericSeries::Poisoned) {
            return Ok(());
        }
        let Some(x) = args.first().and_then(xsd_of).filter(is_numeric_xsd) else {
            self.state = NumericSeries::Poisoned;
            return Ok(());
        };
        if let NumericSeries::Ok(values) = &mut self.state {
            values.push(x);
        } else {
            self.state = NumericSeries::Ok(vec![x]);
        }
        Ok(())
    }

    fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) {
        volatile_combine_unreachable(MEDIAN)
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let Self { state } = *self;
        match state {
            NumericSeries::Empty | NumericSeries::Poisoned => Ok(None),
            NumericSeries::Ok(mut values) => {
                values.sort_by(|a, b| numeric_cmp(a, b).unwrap_or(Ordering::Equal));
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
        Volatility::Volatile
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Associative
    }
    fn state_bound(&self) -> u64 {
        VALUE_PROPORTIONAL_STATE_BOUND
    }
    fn init(&self) -> Box<dyn AggregateAccumulator> {
        Box::new(MedianAccumulator {
            state: NumericSeries::Empty,
        })
    }
}

// ---------------------------------------------------------------------------
// PERCENTILE
// ---------------------------------------------------------------------------

enum PercentileSeries {
    Empty,
    Ok { p: XsdValue, values: Vec<XsdValue> },
    Poisoned,
}

struct PercentileAccumulator {
    state: PercentileSeries,
}

impl AggregateAccumulator for PercentileAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if matches!(self.state, PercentileSeries::Poisoned) {
            return Ok(());
        }
        let (Some(value_term), Some(p_term)) = (args.first(), args.get(1)) else {
            self.state = PercentileSeries::Poisoned;
            return Ok(());
        };
        let Some(p) = xsd_of(p_term).filter(is_numeric_xsd) else {
            self.state = PercentileSeries::Poisoned;
            return Ok(());
        };
        let Some(x) = xsd_of(value_term).filter(is_numeric_xsd) else {
            self.state = PercentileSeries::Poisoned;
            return Ok(());
        };
        let p_mismatch = matches!(
            &self.state,
            PercentileSeries::Ok { p: existing_p, .. } if !numeric_eq(existing_p, &p)
        );
        if p_mismatch {
            self.state = PercentileSeries::Poisoned;
            return Ok(());
        }
        if let PercentileSeries::Ok { values, .. } = &mut self.state {
            values.push(x);
        } else {
            self.state = PercentileSeries::Ok { p, values: vec![x] };
        }
        Ok(())
    }

    fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) {
        volatile_combine_unreachable(PERCENTILE)
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let Self { state } = *self;
        match state {
            PercentileSeries::Empty | PercentileSeries::Poisoned => Ok(None),
            PercentileSeries::Ok { p, mut values } => {
                values.sort_by(|a, b| numeric_cmp(a, b).unwrap_or(Ordering::Equal));
                Ok(percentile_of(&values, &p).as_ref().map(xsd_value_to_term))
            }
        }
    }
}

struct PercentileAggregate;

impl CustomAggregate for PercentileAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(2)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Volatile
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Associative
    }
    fn state_bound(&self) -> u64 {
        VALUE_PROPORTIONAL_STATE_BOUND
    }
    fn init(&self) -> Box<dyn AggregateAccumulator> {
        Box::new(PercentileAccumulator {
            state: PercentileSeries::Empty,
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
// exists to solve TWO problems neither of which applies here. First, it
// avoids catastrophic cancellation under FLOATING-POINT re-association —
// irrelevant to this crate's EXACT decimal/integer tower, where `Σx²` and
// `(Σx)²` are exact integers/decimals with no cancellation error to avoid.
// Second, it is INCREMENTALLY mergeable across parallel partial folds — moot
// here because every member in this family is `Volatility::Volatile` (see the
// module docs' "Why these are `Volatility::Volatile`" section), so exactly
// ONE accumulator ever folds a group, sequentially. What Welford's per-row
// division WOULD cost here, with neither benefit realized, is precision:
// `mean = mean + delta/n` rounds once PER ROW at this crate's 18-fractional-
// digit decimal-division ceiling (`purrdf_xsd::numeric::MAX_DECIMAL_SCALE`),
// so an all-integer group's population variance can drift off its exact
// integer answer by a few units in the 18th digit. The sum/sum-of-squares
// form divides exactly ONCE (twice, for the final `Σx²ᵢ − (Σx)²/n` and once
// more for `/ denom`), so an all-integer or all-decimal group's variance
// stays EXACTLY the textbook answer — see this module's `var_pop_matches_the_
// known_dataset` test, which pins the exact `"4"` this form produces.

#[derive(Clone, Copy)]
enum MomentsKind {
    Stddev,
    StddevPop,
    Variance,
    VarPop,
}

impl MomentsKind {
    const fn local_name(self) -> &'static str {
        match self {
            Self::Stddev => STDDEV,
            Self::StddevPop => STDDEV_POP,
            Self::Variance => VARIANCE,
            Self::VarPop => VAR_POP,
        }
    }
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

    fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) {
        volatile_combine_unreachable(self.kind.local_name())
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
        Volatility::Volatile
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        MOMENTS_STATE_BOUND
    }
    fn init(&self) -> Box<dyn AggregateAccumulator> {
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

    fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) {
        volatile_combine_unreachable(MODE)
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
        Volatility::Volatile
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Associative
    }
    fn state_bound(&self) -> u64 {
        VALUE_PROPORTIONAL_STATE_BOUND
    }
    fn init(&self) -> Box<dyn AggregateAccumulator> {
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

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) {
        // `self` is the earlier chunk: if it already saw a row, its value IS the
        // group's first value regardless of what a later chunk holds. Only an
        // empty earlier chunk defers to the later one.
        if self.value.is_none()
            && let Ok(Some(v)) = other.finish()
        {
            self.value = Some(v);
        }
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
    fn init(&self) -> Box<dyn AggregateAccumulator> {
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

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) {
        // `other` is the later chunk: whatever it holds is later in row order
        // than anything `self` holds, so it always wins when present.
        if let Ok(Some(v)) = other.finish() {
            self.value = Some(v);
        }
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
    fn init(&self) -> Box<dyn AggregateAccumulator> {
        Box::new(LastAccumulator { value: None })
    }
}

// ---------------------------------------------------------------------------
// TOPK
// ---------------------------------------------------------------------------

enum TopKState {
    Empty,
    Ok { k: i128, values: Vec<TermValue> },
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
        if matches!(self.state, TopKState::Poisoned) {
            return Ok(());
        }
        let (Some(value), Some(k_term)) = (args.first(), args.get(1)) else {
            self.state = TopKState::Poisoned;
            return Ok(());
        };
        let Some(XsdValue::Integer { value: k, .. }) = xsd_of(k_term) else {
            self.state = TopKState::Poisoned;
            return Ok(());
        };
        if k <= 0 {
            self.state = TopKState::Poisoned;
            return Ok(());
        }
        let mismatch =
            matches!(&self.state, TopKState::Ok { k: existing_k, .. } if *existing_k != k);
        if mismatch {
            self.state = TopKState::Poisoned;
            return Ok(());
        }
        let k_usize = usize::try_from(k).unwrap_or(usize::MAX);
        if let TopKState::Ok { values, .. } = &mut self.state {
            insert_bounded(values, k_usize, value.clone());
        } else {
            let mut values = Vec::new();
            insert_bounded(&mut values, k_usize, value.clone());
            self.state = TopKState::Ok { k, values };
        }
        Ok(())
    }

    fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) {
        volatile_combine_unreachable(TOPK)
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        let Self { state } = *self;
        let TopKState::Ok { values, .. } = state else {
            return Ok(None);
        };
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
        Arity::Exact(2)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Volatile
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Associative
    }
    fn state_bound(&self) -> u64 {
        TOPK_STATE_BOUND
    }
    fn init(&self) -> Box<dyn AggregateAccumulator> {
        Box::new(TopKAccumulator {
            state: TopKState::Empty,
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
    /// reachable as `AGG(<namespace><LOCAL-NAME>, args…)`.
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

    fn lex(t: &TermValue) -> String {
        match t {
            TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
            other => panic!("expected a literal, got {other:?}"),
        }
    }

    fn fold(local: &str, rows: &[Vec<TermValue>]) -> Option<TermValue> {
        let registry = registry();
        let agg = registry.resolve(&iri(local)).expect("registered");
        let mut acc = agg.init();
        for row in rows {
            acc.step(row).expect("step");
        }
        acc.finish().expect("finish")
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

    // ---- PERCENTILE ------------------------------------------------------------

    #[test]
    fn percentile_zero_is_the_minimum_and_one_is_the_maximum() {
        let rows = |p: TermValue| {
            vec![
                vec![int(1), p.clone()],
                vec![int(2), p.clone()],
                vec![int(3), p],
            ]
        };
        let p0 = fold(PERCENTILE, &rows(dec("0"))).expect("bound");
        assert_eq!(lex(&p0), "1");
        let p1 = fold(PERCENTILE, &rows(dec("1"))).expect("bound");
        assert_eq!(lex(&p1), "3");
    }

    #[test]
    fn percentile_out_of_range_poisons() {
        let rows = vec![vec![int(1), dec("1.5")], vec![int(2), dec("1.5")]];
        assert_eq!(fold(PERCENTILE, &rows), None);
    }

    #[test]
    fn percentile_inconsistent_p_across_rows_poisons() {
        let rows = vec![vec![int(1), dec("0.5")], vec![int(2), dec("0.25")]];
        assert_eq!(fold(PERCENTILE, &rows), None);
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

    #[test]
    fn first_combine_keeps_the_earlier_chunks_value() {
        let registry = registry();
        let agg = registry.resolve(&iri(FIRST)).expect("registered");
        let mut a = agg.init();
        a.step(&[int(1)]).expect("step");
        let mut b = agg.init();
        b.step(&[int(2)]).expect("step");
        a.combine(b);
        assert_eq!(lex(&a.finish().expect("finish").expect("bound")), "1");
    }

    #[test]
    fn last_combine_keeps_the_later_chunks_value() {
        let registry = registry();
        let agg = registry.resolve(&iri(LAST)).expect("registered");
        let mut a = agg.init();
        a.step(&[int(1)]).expect("step");
        let mut b = agg.init();
        b.step(&[int(2)]).expect("step");
        a.combine(b);
        assert_eq!(lex(&a.finish().expect("finish").expect("bound")), "2");
    }

    // ---- TOPK --------------------------------------------------------------------

    #[test]
    fn topk_returns_the_largest_k_values_descending() {
        let rows = [3, 1, 4, 1, 5, 9, 2, 6]
            .into_iter()
            .map(|n| vec![int(n), int(3)])
            .collect::<Vec<_>>();
        let v = fold(TOPK, &rows).expect("bound");
        assert_eq!(lex(&v), "9 6 5");
    }

    #[test]
    fn topk_with_fewer_values_than_k_returns_all_of_them() {
        let rows = vec![vec![int(1), int(5)], vec![int(2), int(5)]];
        let v = fold(TOPK, &rows).expect("bound");
        assert_eq!(lex(&v), "2 1");
    }

    #[test]
    fn topk_non_positive_k_poisons() {
        assert_eq!(fold(TOPK, &[vec![int(1), int(0)]]), None);
        assert_eq!(fold(TOPK, &[vec![int(1), int(-1)]]), None);
    }

    #[test]
    fn topk_non_integer_k_poisons() {
        assert_eq!(fold(TOPK, &[vec![int(1), dec("2.5")]]), None);
    }

    #[test]
    fn topk_inconsistent_k_across_rows_poisons() {
        let rows = vec![vec![int(1), int(2)], vec![int(2), int(3)]];
        assert_eq!(fold(TOPK, &rows), None);
    }

    #[test]
    fn topk_of_empty_group_is_unbound() {
        assert_eq!(fold(TOPK, &[]), None);
    }
}
