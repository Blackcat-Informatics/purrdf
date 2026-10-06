// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The statistics behind a benchmark's estimates: robust location and spread,
//! a seeded bootstrap for their confidence intervals, and outlier fences.
//!
//! * **Quantiles** are Hyndman and Fan's type 7 (linear interpolation between
//!   order statistics, `h = (n - 1) q`), the definition R and NumPy use by
//!   default. The median is the 0.5 quantile: the middle value, or the mean of
//!   the two middle values.
//! * **MAD** is the raw median absolute deviation from the median, unscaled.
//!   Multiply by 1.4826 for a standard-deviation estimate under normality.
//! * **Bootstrap** intervals are percentile intervals over [`RESAMPLES`]
//!   resamples drawn with replacement by `xoshiro256**` seeded through
//!   `purrdf_hash::mix` (SplitMix64), so one seed always yields one interval:
//!   a report is reproducible from its samples.
//! * **Outliers** are classified by Tukey's fences on the interquartile range:
//!   mild beyond 1.5 IQR outside the quartiles, severe beyond 3 IQR. They are
//!   reported, never removed: the median and MAD are already robust to them.
//!
//! Every function here takes at least one value and panics on an empty slice:
//! a benchmark always has samples, so an empty one is a harness defect.

use crate::rng::Xoshiro256;

/// The number of bootstrap resamples behind every interval.
pub const RESAMPLES: usize = 10_000;

/// The two-sided confidence level of every interval.
pub const CONFIDENCE: f64 = 0.95;

/// A closed interval `[low, high]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    /// The lower bound.
    pub low: f64,
    /// The upper bound.
    pub high: f64,
}

/// Outlier counts under Tukey's fences, by side and severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Outliers {
    /// Below `Q1 - 3 IQR`.
    pub low_severe: usize,
    /// In `[Q1 - 3 IQR, Q1 - 1.5 IQR)`.
    pub low_mild: usize,
    /// In `(Q3 + 1.5 IQR, Q3 + 3 IQR]`.
    pub high_mild: usize,
    /// Above `Q3 + 3 IQR`.
    pub high_severe: usize,
}

impl Outliers {
    /// Every outlier, mild and severe, on both sides.
    pub const fn total(&self) -> usize {
        self.low_severe + self.low_mild + self.high_mild + self.high_severe
    }
}

/// `values` sorted ascending by the IEEE 754 total order.
pub fn sorted(values: &[f64]) -> Vec<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted
}

/// The type 7 quantile `q` (in `[0, 1]`) of `sorted`, which must be sorted
/// ascending.
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    assert!(!sorted.is_empty(), "the quantile of no values is undefined");
    assert!((0.0..=1.0).contains(&q), "quantile {q} is outside [0, 1]");
    let h = (sorted.len() - 1) as f64 * q;
    let below = h.floor();
    let index = below as usize;
    let fraction = h - below;
    match sorted.get(index + 1) {
        Some(&next) if fraction > 0.0 => fraction.mul_add(next - sorted[index], sorted[index]),
        _ => sorted[index],
    }
}

/// The median of `values`, in any order.
pub fn median(values: &[f64]) -> f64 {
    quantile(&sorted(values), 0.5)
}

/// The median absolute deviation of `values` from `center` (unscaled).
pub fn median_absolute_deviation(values: &[f64], center: f64) -> f64 {
    let deviations: Vec<f64> = values.iter().map(|value| (value - center).abs()).collect();
    median(&deviations)
}

/// Tukey's fences over `values`, in any order.
pub fn classify_outliers(values: &[f64]) -> Outliers {
    let sorted = sorted(values);
    let q1 = quantile(&sorted, 0.25);
    let q3 = quantile(&sorted, 0.75);
    let iqr = q3 - q1;
    let (low_severe, low_mild) = (iqr.mul_add(-3.0, q1), iqr.mul_add(-1.5, q1));
    let (high_mild, high_severe) = (iqr.mul_add(1.5, q3), iqr.mul_add(3.0, q3));
    let mut outliers = Outliers::default();
    for &value in &sorted {
        if value < low_severe {
            outliers.low_severe += 1;
        } else if value < low_mild {
            outliers.low_mild += 1;
        } else if value > high_severe {
            outliers.high_severe += 1;
        } else if value > high_mild {
            outliers.high_mild += 1;
        }
    }
    outliers
}

/// The median of `values`, reordering them.
fn median_in_place(values: &mut [f64]) -> f64 {
    let odd = values.len() % 2 == 1;
    let middle = values.len() / 2;
    let (lower, &mut upper, _) = values.select_nth_unstable_by(middle, f64::total_cmp);
    if odd {
        upper
    } else {
        let below = lower.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        f64::midpoint(below, upper)
    }
}

/// One resample of `values` with replacement, into `buffer`.
fn resample(values: &[f64], buffer: &mut [f64], rng: &mut Xoshiro256) {
    let last = values.len() as u64 - 1;
    for slot in buffer {
        *slot = values[rng.up_to(last) as usize];
    }
}

/// The percentile interval at `confidence` over the bootstrap statistics.
fn percentile_interval(mut statistics: Vec<f64>, confidence: f64) -> Interval {
    statistics.sort_by(f64::total_cmp);
    let tail = (1.0 - confidence) / 2.0;
    Interval {
        low: quantile(&statistics, tail),
        high: quantile(&statistics, 1.0 - tail),
    }
}

/// The bootstrap percentile interval of the median of `samples`, from
/// `resamples` resamples at `confidence`, seeded at `seed`.
pub fn bootstrap_median(samples: &[f64], resamples: usize, confidence: f64, seed: u64) -> Interval {
    assert!(
        !samples.is_empty(),
        "the bootstrap of no samples is undefined"
    );
    assert!(resamples > 0, "a bootstrap needs at least one resample");
    let mut rng = Xoshiro256::from_seed(seed);
    let mut buffer = vec![0.0; samples.len()];
    let statistics = (0..resamples)
        .map(|_| {
            resample(samples, &mut buffer, &mut rng);
            median_in_place(&mut buffer)
        })
        .collect();
    percentile_interval(statistics, confidence)
}

/// The relative change of the median from `base` to `new`
/// (`median(new) / median(base) - 1`), and its bootstrap percentile interval
/// from `resamples` paired resamples of both sets at `confidence`, seeded at
/// `seed`.
pub fn bootstrap_change(
    new: &[f64],
    base: &[f64],
    resamples: usize,
    confidence: f64,
    seed: u64,
) -> (f64, Interval) {
    assert!(
        !new.is_empty() && !base.is_empty(),
        "a change needs samples on both sides"
    );
    assert!(resamples > 0, "a bootstrap needs at least one resample");
    let point = median(new) / median(base) - 1.0;
    let mut rng = Xoshiro256::from_seed(seed);
    let mut new_buffer = vec![0.0; new.len()];
    let mut base_buffer = vec![0.0; base.len()];
    let statistics = (0..resamples)
        .map(|_| {
            resample(new, &mut new_buffer, &mut rng);
            resample(base, &mut base_buffer, &mut rng);
            median_in_place(&mut new_buffer) / median_in_place(&mut base_buffer) - 1.0
        })
        .collect();
    (point, percentile_interval(statistics, confidence))
}
