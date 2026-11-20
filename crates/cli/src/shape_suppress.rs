// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Small-count suppression for the `shape` figures.
//!
//! A count of one to four prints as `"<5"`. A printed total that is the sum of printed cells
//! would give a hidden cell by subtraction, and two breakdowns of one total bound each other,
//! so the figures are chosen together: [`suppress`] hides further cells until no hidden cell is
//! known to be anything narrower than what its own printed form says.
//!
//! Each printed form is an interval. An exact count is a point, `"<5"` is `[1, 4]`,
//! `"withheld"` is `[5, inf)` and `">=N"` is `[N, inf)`. Every total equals the sum of each
//! of its breakdowns, and bound propagation over those equations gives the interval each
//! hidden cell is confined to by everything printed. A figure is safe when that interval is
//! its own printed one: a `"<5"` cell still ranges over one to four and a withheld or
//! lower-bounded cell has no upper bound.

/// A count below this prints as `"<5"` rather than exactly.
pub(crate) const SMALL_COUNT_THRESHOLD: u64 = 5;

/// No upper bound.
const INF: u64 = u64::MAX;

/// A printed count: exact, suppressed for being small, a lower bound, or withheld.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Count {
    Exact(u64),
    Small,
    AtLeast(u64),
    /// An exact cell of at least the threshold, hidden so that the cells printed beside it
    /// do not pin a suppressed cell.
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

    /// The count of a total that is the sum of the cells of one partition.
    ///
    /// With a single partition a hidden cell leaves the total a lower bound that the exact
    /// cells already give, so nothing more is learned.
    pub(crate) fn total(total: u64, cells: &[Self]) -> Self {
        if (1..SMALL_COUNT_THRESHOLD).contains(&total) {
            return Self::Small;
        }
        if cells.iter().all(|cell| cell.is_exact()) {
            Self::of(total)
        } else {
            Self::AtLeast(exact_sum(cells))
        }
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
    const fn interval(self) -> (u64, u64) {
        match self {
            Self::Exact(n) => (n, n),
            Self::Small => (1, SMALL_COUNT_THRESHOLD - 1),
            Self::Withheld => (SMALL_COUNT_THRESHOLD, INF),
            Self::AtLeast(n) => (n, INF),
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

/// The interval each variable is confined to by every printed form and every equation.
fn derive(vars: &[(Count, u64)], sums: &[Sum]) -> Vec<(u64, u64)> {
    let mut bounds: Vec<(u64, u64)> = vars.iter().map(|var| var.0.interval()).collect();
    for _ in 0..4 * (vars.len() + 1) {
        let before = bounds.clone();
        for sum in sums {
            let lows: u64 = sum
                .cells
                .iter()
                .fold(0, |acc, &c| acc.saturating_add(bounds[c].0));
            let highs: u64 = sum
                .cells
                .iter()
                .fold(0, |acc, &c| acc.saturating_add(bounds[c].1));
            let total = &mut bounds[sum.total];
            total.0 = total.0.max(lows);
            if highs != INF {
                total.1 = total.1.min(highs);
            }
            let (total_low, total_high) = *total;
            for &cell in &sum.cells {
                let others_low = lows.saturating_sub(bounds[cell].0);
                let others_high = sum
                    .cells
                    .iter()
                    .filter(|&&c| c != cell)
                    .fold(0_u64, |acc, &c| acc.saturating_add(bounds[c].1));
                if others_high != INF {
                    bounds[cell].0 = bounds[cell].0.max(total_low.saturating_sub(others_high));
                }
                if total_high != INF {
                    bounds[cell].1 = bounds[cell].1.min(total_high.saturating_sub(others_low));
                }
            }
        }
        if bounds == before {
            break;
        }
    }
    bounds
}

/// Whether the figures pin `count` to something narrower than its printed form.
const fn pinned(count: Count, bound: (u64, u64)) -> bool {
    match count {
        Count::Exact(_) => false,
        Count::Small => bound.0 != 1 || bound.1 != SMALL_COUNT_THRESHOLD - 1,
        Count::Withheld | Count::AtLeast(_) => bound.1 != INF,
    }
}

/// Choose how every variable prints so that no hidden one is pinned.
///
/// Each variable is its printed form and its true value; a variable in `fixed` keeps the form
/// it was given and is never recomputed. Totals start as exact when every
/// cell of every breakdown is exact and as the lower bound the exact cells give otherwise;
/// then exact cells are withheld, smallest first, until no hidden variable is pinned. Only
/// the variables in `candidates` may be withheld when it is given.
///
/// Returns whether it ended with no hidden variable pinned. A caller that gets `false` must
/// not print the figures as they stand.
#[must_use]
pub(crate) fn suppress(
    vars: &mut [(Count, u64)],
    sums: &[Sum],
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
    for _ in 0..=vars.len() {
        let bounds = derive(vars, sums);
        let Some(violation) = (0..vars.len()).find(|&i| pinned(vars[i].0, bounds[i])) else {
            return true;
        };
        // Prefer a breakdown that bounds everything above, then any breakdown that touches
        // the pinned variable, then any at all.
        let touching: Vec<&Sum> = sums
            .iter()
            .filter(|sum| sum.total == violation || sum.cells.contains(&violation))
            .collect();
        let bounded = |sum: &&Sum| sum.cells.iter().all(|&c| vars[c].0.interval().1 != INF);
        let pick = |from: &[&Sum]| {
            from.iter()
                .flat_map(|sum| sum.cells.iter().copied())
                .filter(|&c| candidates.is_none_or(|only| only.contains(&c)))
                .filter(|&c| matches!(vars[c].0, Count::Exact(n) if n > 0))
                .min_by_key(|&c| vars[c].1)
        };
        let all: Vec<&Sum> = sums.iter().collect();
        let tight: Vec<&Sum> = touching.iter().copied().filter(bounded).collect();
        let loose: Vec<&Sum> = all.iter().copied().filter(bounded).collect();
        let Some(cell) = pick(&tight)
            .or_else(|| pick(&touching))
            .or_else(|| pick(&loose))
            .or_else(|| pick(&all))
        else {
            return false;
        };
        vars[cell].0 = Count::Withheld;
    }
    let bounds = derive(vars, sums);
    !(0..vars.len()).any(|i| pinned(vars[i].0, bounds[i]))
}

#[cfg(test)]
mod tests {
    use super::{Count, Sum, suppress};

    /// P1: the hidden length bin and the withheld integer count are not recoverable from the
    /// two breakdowns of ten literals.
    #[test]
    fn two_breakdowns_do_not_pin_a_hidden_cell() {
        // total; length bins 4 and 6; datatype cells integer 5 and string 5.
        let mut vars = vec![
            (Count::of(10), 10),
            (Count::of(4), 4),
            (Count::of(6), 6),
            (Count::of(5), 5),
            (Count::of(5), 5),
        ];
        let sums = [
            Sum {
                total: 0,
                cells: vec![1, 2],
            },
            Sum {
                total: 0,
                cells: vec![3, 4],
            },
        ];
        assert!(suppress(&mut vars, &sums, &[], None));
        let bounds = super::derive(&vars, &sums);
        for (index, var) in vars.iter().enumerate() {
            assert!(!super::pinned(var.0, bounds[index]), "{index}");
        }
        assert_eq!(vars[1].0, Count::Small);
        assert!(!vars[0].0.is_exact());
    }
}
