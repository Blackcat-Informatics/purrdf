// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Binary64 proposals for the certified inverse solver.
//!
//! Nothing in this module is a proof. It evaluates the same auxiliary-sphere
//! longitude equation as the certified solver with ordinary round-to-nearest
//! binary64 arithmetic, and runs a safeguarded Newton iteration on it. The
//! result only chooses *where* the certified interval law is evaluated: the
//! caller accepts an azimuth only after an outward interval Newton inclusion
//! over a neighbourhood proves the unique root lies there.
//!
//! Every operation is a correctly rounded IEEE basic operation obtained from a
//! validated [`Binary64`] chunk, and the sine, cosine and arctangent below use
//! only those operations. The proposal is therefore the same on every host,
//! the x87 included, so the certified evaluation points and the enclosures
//! built from them are host-independent too.

use purrdf_xsd::ieee::Binary64;

/// Exact-recurrence Taylor and arctangent coefficients plus binary64 pi.
#[derive(Clone, Debug)]
pub(super) struct Tables {
    pi: f64,
    sine: [f64; 10],
    cosine: [f64; 10],
    arctangent: [f64; 17],
}

impl Tables {
    /// Generate `(-1)^k/(2k+1)!`, `(-1)^k/(2k)!` and `(-1)^k/(2k+1)` by their
    /// recurrences. The degree-19 sine and degree-18 cosine remainders are below
    /// 2^-60 on |x| <= pi/4; the degree-33 arctangent remainder is below 2^-70.
    pub(super) fn new(pi: f64, ops: Binary64<'_>) -> Self {
        let mut sine = [0.0_f64; 10];
        let mut cosine = [0.0_f64; 10];
        let mut arctangent = [0.0_f64; 17];
        let (mut odd, mut even) = (1.0_f64, 1.0_f64);
        for k in 0..10_u32 {
            let positive = k % 2 == 0;
            cosine[k as usize] = signed(even, positive);
            sine[k as usize] = signed(odd, positive);
            even = ops.div(even, ops.mul(f64::from(2 * k + 1), f64::from(2 * k + 2)));
            odd = ops.div(odd, ops.mul(f64::from(2 * k + 2), f64::from(2 * k + 3)));
        }
        for k in 0..17_u32 {
            arctangent[k as usize] = signed(ops.div(1.0, f64::from(2 * k + 1)), k % 2 == 0);
        }
        Self {
            pi,
            sine,
            cosine,
            arctangent,
        }
    }

    /// Deterministic sine and cosine after quarter-turn reduction.
    pub(super) fn sin_cos(&self, x: f64, ops: Binary64<'_>) -> Option<(f64, f64)> {
        if !x.is_finite() || x.abs() > 64.0 {
            return None;
        }
        let quarter = ops.mul(self.pi, 0.5);
        let turns = ops.div(x, quarter).round_ties_even();
        let reduced = ops.sub(x, ops.mul(turns, quarter));
        let square = ops.mul(reduced, reduced);
        let mut sine = 0.0_f64;
        let mut cosine = 0.0_f64;
        for k in (0..10).rev() {
            sine = ops.add(ops.mul(sine, square), self.sine[k]);
            cosine = ops.add(ops.mul(cosine, square), self.cosine[k]);
        }
        let sine = ops.mul(sine, reduced);
        // |turns| <= 41 on the admitted range, so this conversion is exact.
        #[allow(clippy::cast_possible_truncation)]
        let quadrant = (turns as i64).rem_euclid(4);
        Some(match quadrant {
            0 => (sine, cosine),
            1 => (cosine, negate(sine)),
            2 => (negate(sine), negate(cosine)),
            _ => (negate(cosine), sine),
        })
    }

    /// Deterministic principal arctangent of `y/x`; the origin has no value.
    pub(super) fn atan2(&self, y: f64, x: f64, ops: Binary64<'_>) -> Option<f64> {
        if !y.is_finite() || !x.is_finite() || (x == 0.0 && y == 0.0) {
            return None;
        }
        let half = ops.mul(self.pi, 0.5);
        let (ax, ay) = (x.abs(), y.abs());
        // The arctangent of the smaller over the larger magnitude is in [0, pi/4].
        let base = if ay <= ax {
            self.atan_unit(ops.div(ay, ax), ops)
        } else {
            ops.sub(half, self.atan_unit(ops.div(ax, ay), ops))
        };
        let angle = if x.is_sign_negative() && x != 0.0 {
            ops.sub(self.pi, base)
        } else {
            base
        };
        Some(signed(angle, !(y.is_sign_negative() && y != 0.0)))
    }

    /// Arctangent on [0, 1]: two half-angle reductions give |t| <= tan(pi/16).
    fn atan_unit(&self, value: f64, ops: Binary64<'_>) -> f64 {
        let mut reduced = value;
        for _ in 0..2 {
            reduced = ops.div(
                reduced,
                ops.add(1.0, ops.sqrt(ops.add(1.0, ops.mul(reduced, reduced)))),
            );
        }
        let square = ops.mul(reduced, reduced);
        let mut sum = 0.0_f64;
        for &coefficient in self.arctangent.iter().rev() {
            sum = ops.add(ops.mul(sum, square), coefficient);
        }
        ops.mul(ops.mul(sum, reduced), 4.0)
    }
}

/// Exact sign change, without floating arithmetic.
use purrdf_xsd::ieee::f64_negate as negate;

fn signed(value: f64, positive: bool) -> f64 {
    if positive { value } else { negate(value) }
}

/// Reduced-latitude endpoint data of one canonical inverse problem.
// The flags mirror the certified solver's independent symmetry predicates.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug)]
pub(super) struct Sphere {
    pub(super) sin1: f64,
    pub(super) cos1: f64,
    pub(super) sin2: f64,
    pub(super) cos2: f64,
    pub(super) h1: f64,
    pub(super) h2: f64,
    pub(super) same_latitude: bool,
    pub(super) opposite_latitude: bool,
    pub(super) short: bool,
    pub(super) polar: bool,
}

/// Midpoints of the generated auxiliary-integral coefficients.
#[derive(Clone, Debug)]
pub(super) struct Series {
    pub(super) c: f64,
    pub(super) ep2: f64,
    pub(super) e2: f64,
    pub(super) square_root: [f64; 9],
    pub(super) reciprocal: [f64; 9],
    pub(super) order: usize,
    pub(super) tables: Tables,
}

/// One proposal of the longitude equation and its azimuth derivative.
struct Sample {
    longitude: f64,
    derivative: f64,
}

/// The longitude equation of [`super::hybrid_with_trig`], in round-to-nearest
/// binary64. `None` marks a configuration this proposal does not model.
fn sample(sphere: &Sphere, series: &Series, alpha: f64, ops: Binary64<'_>) -> Option<Sample> {
    let trig = &series.tables;
    let (sin_alpha, cos_alpha) = trig.sin_cos(alpha, ops)?;
    let sin0 = ops.mul(sin_alpha, sphere.cos1);
    let x1 = ops.mul(cos_alpha, sphere.cos1);
    let cos0_squared = ops.add(ops.mul(x1, x1), ops.mul(sphere.sin1, sphere.sin1));
    let x2 = if sphere.same_latitude || sphere.opposite_latitude {
        x1.abs()
    } else {
        let delta = if sphere.polar {
            ops.mul(
                ops.sub(sphere.cos2, sphere.cos1),
                ops.add(sphere.cos2, sphere.cos1),
            )
        } else {
            ops.mul(
                ops.sub(sphere.sin1, sphere.sin2),
                ops.add(sphere.sin1, sphere.sin2),
            )
        };
        ops.sqrt(ops.add(ops.mul(x1, x1), delta).max(0.0))
    };
    if sphere.opposite_latitude && cos_alpha <= 0.0 {
        return None;
    }
    let sigma1 = trig.atan2(sphere.sin1, x1, ops)?;
    let sigma2 = trig.atan2(sphere.sin2, x2, ops)?;
    let sine_difference = ops.sub(sphere.sin2, sphere.sin1);
    let cross = if sphere.short && x1 > 0.0 {
        let x_difference = ops.div(
            ops.mul(negate(sine_difference), ops.add(sphere.sin1, sphere.sin2)),
            ops.add(x1, x2),
        );
        ops.sub(
            ops.mul(sine_difference, x1),
            ops.mul(x_difference, sphere.sin1),
        )
    } else {
        ops.sub(ops.mul(sphere.sin2, x1), ops.mul(x2, sphere.sin1))
    };
    let omega = trig.atan2(
        ops.mul(sin0, cross).max(0.0),
        ops.add(
            ops.mul(x1, x2),
            ops.mul(ops.mul(ops.mul(sin0, sin0), sphere.sin1), sphere.sin2),
        ),
        ops,
    )?;
    let delta = if sphere.short {
        trig.atan2(
            cross.max(0.0),
            ops.add(ops.mul(x1, x2), ops.mul(sphere.sin1, sphere.sin2)),
            ops,
        )?
    } else {
        ops.sub(sigma2, sigma1)
    };
    let q = ops.mul(series.ep2, cos0_squared);
    let (longitude_integral, jacobi) = integrals(series, sigma1, sigma2, delta, q, ops)?;
    let correction = ops.mul(ops.mul(series.e2, sin0), longitude_integral);
    let reduced = if sphere.short {
        let h_difference = ops.div(
            ops.mul(
                ops.mul(series.ep2, sine_difference),
                ops.add(sphere.sin1, sphere.sin2),
            ),
            ops.add(sphere.h1, sphere.h2),
        );
        ops.div(
            ops.sub(
                ops.add(
                    ops.mul(sphere.h2, cross),
                    ops.mul(ops.mul(h_difference, sphere.sin1), x2),
                ),
                ops.mul(ops.mul(x1, x2), jacobi),
            ),
            cos0_squared,
        )
    } else {
        ops.div(
            ops.sub(
                ops.sub(
                    ops.mul(ops.mul(sphere.h2, x1), sphere.sin2),
                    ops.mul(ops.mul(sphere.h1, sphere.sin1), x2),
                ),
                ops.mul(ops.mul(x1, x2), jacobi),
            ),
            cos0_squared,
        )
    };
    if x2 <= 0.0 {
        return None;
    }
    let derivative = ops.div(ops.mul(series.c, reduced), x2);
    let longitude = ops.sub(omega, correction);
    (longitude.is_finite() && derivative.is_finite()).then_some(Sample {
        longitude,
        derivative,
    })
}

/// Longitude and Jacobi integrals of the certified sine-power recurrence.
fn integrals(
    series: &Series,
    sigma1: f64,
    sigma2: f64,
    delta: f64,
    q: f64,
    ops: Binary64<'_>,
) -> Option<(f64, f64)> {
    let trig = &series.tables;
    if q == 0.0 {
        return Some((ops.mul(delta, series.reciprocal[0]), 0.0));
    }
    if ops.mul(q, 2.0) >= 1.0 {
        return None;
    }
    let (sin1, cos1) = trig.sin_cos(sigma1, ops)?;
    let (sin2, cos2) = trig.sin_cos(sigma2, ops)?;
    let sin1_squared = ops.mul(sin1, sin1);
    let sin2_squared = ops.mul(sin2, sin2);
    let mut endpoint2 = ops.mul(sin2, cos2);
    let (mut endpoint_difference, square_difference) = if delta.abs() < 0.015_625 {
        let (sin_delta, _) = trig.sin_cos(delta, ops)?;
        let cosine_sum = ops.sub(ops.mul(cos1, cos2), ops.mul(sin1, sin2));
        let sine_sum = ops.add(ops.mul(sin1, cos2), ops.mul(cos1, sin2));
        (
            ops.mul(negate(sin_delta), cosine_sum),
            ops.mul(negate(sin_delta), sine_sum),
        )
    } else {
        (
            ops.sub(ops.mul(sin1, cos1), endpoint2),
            ops.sub(sin1_squared, sin2_squared),
        )
    };
    let mut integral = delta;
    let mut q_power = 1.0_f64;
    let mut longitude = ops.mul(delta, series.reciprocal[0]);
    let mut jacobi = 0.0_f64;
    for n in 1..=series.order {
        q_power = ops.mul(q_power, q);
        let odd = f64::from(u32::try_from(2 * n - 1).ok()?);
        let even = f64::from(u32::try_from(2 * n).ok()?);
        integral = ops.div(ops.add(ops.mul(integral, odd), endpoint_difference), even);
        let term = ops.mul(integral, q_power);
        jacobi = ops.add(jacobi, ops.mul(ops.mul(term, series.square_root[n]), even));
        longitude = ops.add(longitude, ops.mul(term, series.reciprocal[n]));
        endpoint_difference = ops.add(
            ops.mul(endpoint_difference, sin1_squared),
            ops.mul(endpoint2, square_difference),
        );
        endpoint2 = ops.mul(endpoint2, sin2_squared);
    }
    Some((longitude, jacobi))
}

/// A converged proposal: the azimuth and the magnitude of its final Newton step.
#[derive(Clone, Copy, Debug)]
pub(super) struct Proposal {
    pub(super) azimuth: f64,
    pub(super) step: f64,
    /// The proposal derivative of longitude with respect to azimuth.
    pub(super) derivative: f64,
}

/// Safeguarded Newton iteration for `longitude(alpha) = target` on the open
/// bracket `(lower, upper)`, on which the certified solver's longitude law is
/// increasing. Bisection replaces any step leaving the current bracket.
pub(super) fn newton(
    sphere: &Sphere,
    series: &Series,
    target: f64,
    lower: f64,
    upper: f64,
    start: f64,
    ops: Binary64<'_>,
) -> Option<Proposal> {
    let (mut low, mut high) = (lower, upper);
    if low.partial_cmp(&high) != Some(core::cmp::Ordering::Less) {
        return None;
    }
    let mut alpha = if start > low && start < high {
        start
    } else {
        ops.mul(ops.add(low, high), 0.5)
    };
    for _ in 0..24 {
        let value = sample(sphere, series, alpha, ops)?;
        let residual = ops.sub(value.longitude, target);
        if residual < 0.0 {
            low = alpha;
        } else if residual > 0.0 {
            high = alpha;
        } else if value.derivative > 0.0 {
            return Some(Proposal {
                azimuth: alpha,
                step: 0.0,
                derivative: value.derivative,
            });
        }
        let mut next = if value.derivative > 0.0 {
            ops.sub(alpha, ops.div(residual, value.derivative))
        } else {
            f64::NAN
        };
        if !(next > low && next < high) {
            next = ops.mul(ops.add(low, high), 0.5);
        }
        let step = ops.sub(next, alpha).abs();
        alpha = next;
        if step <= ops.mul(alpha.abs().max(0.000_976_562_5), 4.440_892_098_500_626e-16)
            && value.derivative > 0.0
        {
            return Some(Proposal {
                azimuth: alpha,
                step,
                derivative: value.derivative,
            });
        }
    }
    None
}
