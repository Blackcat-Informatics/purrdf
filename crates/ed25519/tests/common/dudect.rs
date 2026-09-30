// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A dudect-style statistical constant-time check ("Dude, is my code constant
//! time?", Reparaz, Balasch and Verbauwhede, 2017).
//!
//! Two classes of input (a fixed secret and fresh random secrets) are run
//! through the operation under test, the class of each sample chosen by a
//! random coin so that interleaving spreads clock drift and machine load over
//! both classes alike. Each run is timed with `Instant`. As in dudect, the
//! timings are also cropped at a ladder of upper percentiles of the pooled
//! distribution (removing the scheduler and interrupt outliers that swamp a
//! small, real difference), and Welch's t-test is applied to the uncropped and
//! to every cropped set; the statistic reported is the largest |t|.
//!
//! The verdict follows dudect's threshold: |t| below [`THRESHOLD`] is "no
//! evidence of a leak". A run on a loaded host can be pushed over it by noise
//! that has nothing to do with the code, whereas a real timing dependence
//! grows with the square root of the sample count and does so on every run;
//! so an operation is judged over up to [`ATTEMPTS`] independent runs (a fresh
//! random stream each) and passes at the first run under the threshold, and
//! is reported as leaking only when every run is over it.

use std::hint::black_box;
use std::time::Instant;

use purrdf_testkit::rng::SplitMix64;

/// dudect's decision threshold on |t|.
pub(crate) const THRESHOLD: f64 = 10.0;
/// Independent runs an operation may take to get under [`THRESHOLD`].
pub(crate) const ATTEMPTS: usize = 3;

/// Crop points: the pooled-timing percentiles below which samples are kept
/// (the last, 1.0, is no cropping).
const CROPS: [f64; 10] = [0.5, 0.65, 0.75, 0.85, 0.9, 0.95, 0.975, 0.99, 0.995, 1.0];

#[derive(Default)]
struct Moments {
    n: f64,
    mean: f64,
    m2: f64,
}

impl Moments {
    /// Welford's update.
    fn push(&mut self, x: f64) {
        self.n += 1.0;
        let delta = x - self.mean;
        self.mean += delta / self.n;
        self.m2 = delta.mul_add(x - self.mean, self.m2);
    }

    fn variance(&self) -> f64 {
        self.m2 / (self.n - 1.0)
    }
}

/// Welch's t between two classes of timings.
fn welch(a: &Moments, b: &Moments) -> f64 {
    let denom = (a.variance() / a.n + b.variance() / b.n).sqrt();
    if denom == 0.0 {
        0.0
    } else {
        (a.mean - b.mean) / denom
    }
}

/// The largest |t| over the crop ladder for `(class, nanoseconds)` samples.
fn max_abs_t(samples: &[(bool, f64)]) -> f64 {
    let mut pooled: Vec<f64> = samples.iter().map(|&(_, t)| t).collect();
    pooled.sort_by(f64::total_cmp);
    let mut worst = 0.0f64;
    for crop in CROPS {
        let index = ((pooled.len() as f64 * crop) as usize).min(pooled.len()) - 1;
        let limit = pooled[index];
        let (mut fixed, mut random) = (Moments::default(), Moments::default());
        for &(class, t) in samples {
            if t <= limit {
                if class { &mut fixed } else { &mut random }.push(t);
            }
        }
        worst = worst.max(welch(&fixed, &random).abs());
    }
    worst
}

/// One run: `count` samples, each a coin-chosen class. `fixed` builds the
/// fixed-class input, `random` a fresh random-class input from the stream;
/// inputs are built before the clock starts. Returns the largest |t|.
pub(crate) fn run<I>(
    count: usize,
    seed: u64,
    fixed: impl Fn() -> I,
    random: impl Fn(&mut SplitMix64) -> I,
    operation: impl Fn(&I),
) -> f64 {
    let mut rng = SplitMix64::new(seed);
    // Warm caches, the branch predictors and the CPU governor.
    for _ in 0..256 {
        operation(&fixed());
    }
    let mut samples = Vec::with_capacity(count);
    for _ in 0..count {
        let class = rng.next_u64() & 1 == 1;
        let input = if class { fixed() } else { random(&mut rng) };
        let start = Instant::now();
        operation(black_box(&input));
        let elapsed = start.elapsed().as_nanos() as f64;
        samples.push((class, elapsed));
    }
    max_abs_t(&samples)
}

/// Judge `operation`: up to [`ATTEMPTS`] runs of `count` samples each, stopping
/// at the first under [`THRESHOLD`]. Returns every run's |t| and whether the
/// operation passed.
pub(crate) fn judge<I>(
    name: &str,
    count: usize,
    fixed: impl Fn() -> I,
    random: impl Fn(&mut SplitMix64) -> I,
    operation: impl Fn(&I),
) -> (Vec<f64>, bool) {
    let mut ts = Vec::new();
    for attempt in 0..ATTEMPTS {
        let t = run(
            count,
            0xD0D0_0000 + attempt as u64,
            &fixed,
            &random,
            &operation,
        );
        println!(
            "dudect {name}: attempt {} of {ATTEMPTS}, {count} samples, max |t| = {t:.2}",
            attempt + 1
        );
        ts.push(t);
        if t < THRESHOLD {
            return (ts, true);
        }
    }
    (ts, false)
}
