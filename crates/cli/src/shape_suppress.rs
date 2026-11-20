// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Small-count suppression for the `shape` figures.
//!
//! A count of one to four prints as `"<5"`. Every printed count is also tied to others: a
//! total equals the sum of each of its breakdowns, and structural facts of an RDF 1.2 dataset
//! bound one count by others (a literal is the object of some statement, a parent is a blank
//! node, and so on). A reader who knows those ties can combine the printed figures, so the
//! figures are chosen together: [`suppress`] treats each printed form as an interval,
//! propagates every tie to find what a reader learns, and hides further counts until no
//! hidden count is narrowed to fewer values than its own printed form leaves it.
//!
//! An exact count is a point, `"<5"` is `[1, 4]`, `"withheld"` is `[5, inf)` and `">=N"` is
//! `[N, inf)`. Each tie is a linear inequality over the counts, and an equality is two
//! inequalities. A hidden `"<5"` count is safe when its upper bound stays four and at least
//! three values remain possible for it, and a withheld or lower-bounded count when at least
//! two do.

/// A count below this prints as `"<5"` rather than exactly.
pub(crate) const SMALL_COUNT_THRESHOLD: u64 = 5;

/// No upper bound.
const INF: i128 = i128::MAX / 4;

/// The values a hidden `"<5"` count must keep possible.
const SMALL_KEEPS: i128 = 3;

/// The values a withheld or lower-bounded count must keep possible.
const LARGE_KEEPS: i128 = 2;

/// A printed count: exact, suppressed for being small, a lower bound, or withheld.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Count {
    Exact(u64),
    Small,
    AtLeast(u64),
    /// An exact count of at least the threshold, hidden so that the counts printed beside it
    /// do not pin a hidden one.
    Withheld,
}

impl Count {
    /// `count` as itself, or suppressed when it is one to four.
    pub(crate) const fn of(count: u64) -> Self {
        if count >= 1 && count < SMALL_COUNT_THRESHOLD {
            Self::Small
        } else {
            Self::Exact(count)
        }
    }

    pub(crate) const fn is_exact(self) -> bool {
        matches!(self, Self::Exact(_))
    }

    pub(crate) fn text(self) -> String {
        match self {
            Self::Exact(n) => n.to_string(),
            Self::Small => format!("\"<{SMALL_COUNT_THRESHOLD}\""),
            Self::AtLeast(n) => format!("\">={n}\""),
            Self::Withheld => "\"withheld\"".to_owned(),
        }
    }

    /// The values the printed form allows.
    fn interval(self) -> (i128, i128) {
        match self {
            Self::Exact(n) => (i128::from(n), i128::from(n)),
            Self::Small => (1, i128::from(SMALL_COUNT_THRESHOLD) - 1),
            Self::Withheld => (i128::from(SMALL_COUNT_THRESHOLD), INF),
            Self::AtLeast(n) => (i128::from(n), INF),
        }
    }
}

fn exact_sum(cells: &[Count]) -> u64 {
    cells
        .iter()
        .map(|cell| match cell {
            Count::Exact(n) => *n,
            _ => 0,
        })
        .fold(0, u64::saturating_add)
}

/// One equation: the variable `total` is the sum of the variables `cells`.
#[derive(Clone)]
pub(crate) struct Sum {
    pub(crate) total: usize,
    pub(crate) cells: Vec<usize>,
}

/// One tie: the sum of `coefficient * variable` over `terms` is at most `limit`.
#[derive(Clone)]
pub(crate) struct Linear {
    terms: Vec<(i128, usize)>,
    limit: i128,
}

impl Linear {
    /// The tie `sum(lesser) + constant <= sum(greater)`, each side a list of
    /// `(coefficient, variable)`.
    pub(crate) fn at_most(
        lesser: &[(i128, usize)],
        constant: i128,
        greater: &[(i128, usize)],
    ) -> Self {
        let mut terms: Vec<(i128, usize)> = lesser.to_vec();
        terms.extend(greater.iter().map(|(c, v)| (-c, *v)));
        Self {
            terms,
            limit: -constant,
        }
    }
}

/// Every tie of `sums` as inequalities, followed by `ties`.
fn linearize(sums: &[Sum], ties: &[Linear]) -> Vec<Linear> {
    let mut all = Vec::new();
    for sum in sums {
        let cells: Vec<(i128, usize)> = sum.cells.iter().map(|&c| (1, c)).collect();
        all.push(Linear::at_most(&cells, 0, &[(1, sum.total)]));
        all.push(Linear::at_most(&[(1, sum.total)], 0, &cells));
    }
    all.extend(ties.iter().cloned());
    all
}

/// The interval each variable is confined to by every printed form and every tie.
fn derive(vars: &[(Count, u64)], constraints: &[Linear]) -> Vec<(i128, i128)> {
    let mut low: Vec<i128> = Vec::with_capacity(vars.len());
    let mut high: Vec<i128> = Vec::with_capacity(vars.len());
    for var in vars {
        let (l, h) = var.0.interval();
        low.push(l);
        high.push(h);
    }
    for _ in 0..8 * (vars.len() + 1) {
        let mut changed = false;
        for constraint in constraints {
            for (position, &(coefficient, variable)) in constraint.terms.iter().enumerate() {
                if coefficient == 0 {
                    continue;
                }
                let mut rest_minimum = 0_i128;
                let mut unbounded = false;
                for (other, &(a, v)) in constraint.terms.iter().enumerate() {
                    if other == position {
                        continue;
                    }
                    if a > 0 {
                        rest_minimum += a * low[v];
                    } else if high[v] >= INF {
                        unbounded = true;
                        break;
                    } else {
                        rest_minimum += a * high[v];
                    }
                }
                if unbounded {
                    continue;
                }
                let bound = constraint.limit - rest_minimum;
                if coefficient > 0 {
                    let new_high = bound.div_euclid(coefficient);
                    if new_high < high[variable] {
                        high[variable] = new_high;
                        changed = true;
                    }
                } else {
                    let new_low = -bound.div_euclid(-coefficient);
                    if new_low > low[variable] {
                        low[variable] = new_low;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    low.into_iter().zip(high).collect()
}

/// Whether the figures pin `count` to fewer values than its printed form should leave.
fn pinned(count: Count, bound: (i128, i128)) -> bool {
    let values = bound.1.saturating_sub(bound.0).saturating_add(1);
    match count {
        Count::Exact(_) => false,
        Count::Small => bound.1 < i128::from(SMALL_COUNT_THRESHOLD) - 1 || values < SMALL_KEEPS,
        Count::Withheld | Count::AtLeast(_) => values < LARGE_KEEPS,
    }
}

/// Hide the exact count `variable`: a small one as `"<5"` and any other as withheld.
fn hide(vars: &mut [(Count, u64)], variable: usize) {
    vars[variable].0 = if vars[variable].1 < SMALL_COUNT_THRESHOLD {
        Count::Small
    } else {
        Count::Withheld
    };
}

/// Choose how every variable prints so that no hidden one is pinned.
///
/// Each variable is its printed form and its true value; a variable in `fixed` keeps the form
/// it was given. Totals start as exact when every cell of every breakdown is exact and as the
/// lower bound the exact cells give otherwise; then exact non-zero counts are hidden, smallest
/// first and nearest the pinned count first, until no hidden count is pinned. Only the
/// variables in `candidates` may be hidden when it is given.
///
/// Returns whether it ended with no hidden count pinned. A caller that gets `false` must not
/// print the figures as they stand.
#[must_use]
pub(crate) fn suppress(
    vars: &mut [(Count, u64)],
    sums: &[Sum],
    ties: &[Linear],
    fixed: &[usize],
    candidates: Option<&[usize]>,
) -> bool {
    let mut settled: Vec<usize> = fixed.to_vec();
    for sum in sums.iter().rev() {
        if settled.contains(&sum.total) {
            continue;
        }
        settled.push(sum.total);
        let parts: Vec<Vec<Count>> = sums
            .iter()
            .filter(|other| other.total == sum.total)
            .map(|other| other.cells.iter().map(|&c| vars[c].0).collect())
            .collect();
        let truth = vars[sum.total].1;
        vars[sum.total].0 = if (1..SMALL_COUNT_THRESHOLD).contains(&truth) {
            Count::Small
        } else if parts.iter().flatten().all(|cell| cell.is_exact()) {
            Count::of(truth)
        } else {
            Count::AtLeast(
                parts
                    .iter()
                    .map(|cells| exact_sum(cells))
                    .max()
                    .unwrap_or(0),
            )
        };
    }
    let constraints = linearize(sums, ties);
    for _ in 0..=vars.len() {
        let bounds = derive(vars, &constraints);
        let Some(violation) = (0..vars.len()).find(|&i| pinned(vars[i].0, bounds[i])) else {
            return true;
        };
        let hideable = |c: usize| {
            candidates.is_none_or(|only| only.contains(&c))
                && matches!(vars[c].0, Count::Exact(n) if n > 0)
        };
        let nearest = |from: &mut dyn Iterator<Item = &Linear>| {
            from.flat_map(|constraint| constraint.terms.iter().map(|term| term.1))
                .filter(|&c| hideable(c))
                .min_by_key(|&c| vars[c].1)
        };
        let Some(cell) = nearest(
            &mut constraints
                .iter()
                .filter(|c| c.terms.iter().any(|term| term.1 == violation)),
        )
        .or_else(|| nearest(&mut constraints.iter()))
        .or_else(|| {
            (0..vars.len())
                .filter(|&c| hideable(c))
                .min_by_key(|&c| vars[c].1)
        }) else {
            return false;
        };
        hide(vars, cell);
    }
    let bounds = derive(vars, &constraints);
    !(0..vars.len()).any(|i| pinned(vars[i].0, bounds[i]))
}

#[cfg(test)]
mod tests {
    use super::{Count, Linear, Sum, derive, pinned, suppress};

    /// Two breakdowns of ten literals leave neither the hidden length bin nor the withheld
    /// integer count recoverable, and a literal count bounded by the statements that hold
    /// them is hidden rather than printed beside them.
    #[test]
    fn ties_between_printed_counts_do_not_pin_a_hidden_count() {
        // literals 6; statements 6; length bins 1 and 5; datatype cell integer 6.
        let mut vars = vec![
            (Count::of(6), 6),
            (Count::of(6), 6),
            (Count::of(1), 1),
            (Count::of(5), 5),
            (Count::of(6), 6),
        ];
        let sums = [
            Sum {
                total: 0,
                cells: vec![2, 3],
            },
            Sum {
                total: 0,
                cells: vec![4],
            },
        ];
        // literals <= statements
        let ties = [Linear::at_most(&[(1, 0)], 0, &[(1, 1)])];
        assert!(suppress(&mut vars, &sums, &ties, &[], None));
        let constraints = super::linearize(&sums, &ties);
        let bounds = derive(&vars, &constraints);
        for (index, var) in vars.iter().enumerate() {
            assert!(!pinned(var.0, bounds[index]), "{index}");
        }
        assert_eq!(vars[2].0, Count::Small);
    }
}
