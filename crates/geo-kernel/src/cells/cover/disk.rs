// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The conservative physical-disk classifier.
//!
//! A cell's complete assigned footprint is enclosed by an angular cap about the
//! normalized chart midpoint `u`: its radius is the largest angle from `u` to
//! the four corners of the closed chart rectangle on the normal sphere, plus the
//! `2^-59` assignment guard. The rectangle's edges are great-circle arcs, so it
//! is spherically convex, and angle from `u` is quasi-convex there; its maximum
//! is therefore attained at a corner. An exact surface point `M` near `u`, the
//! binary64 midpoint of `u`'s enclosed latitude and longitude, carries the same
//! cap widened by those enclosures' widths.
//!
//! Two certified bounds then classify the whole footprint against the closed
//! disk of exact radius `r` about the centre `C`:
//!
//! * the normal-sphere bound `m*theta <= d <= R*theta`, with `m = b^2/a` and
//!   `R = a^2/b` the extreme principal radii, applied to the angle between the
//!   centre and footprint normals; and
//! * the metric bound `|d(C,P) - d(C,M)| <= d(M,P) <= R*angle(M,P)`, applied
//!   to a certified binary64 enclosure of the true shortest distance `d(C,M)`
//!   with `M`'s widened cap.
//!
//! The cell is outside when a lower bound exceeds `r`, inside when an upper bound
//! is at most `r`, and straddling otherwise. Every quantity is an outward
//! binary64 interval computed with correctly rounded basic operations and the
//! shared deterministic transcendental kernels, so the classification of a cell
//! is the same on every host; it does not depend on caller limits.

use purrdf_xsd::BigInt;
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::math::{
    CheckedBinary64, CoordinateMath, FixedInterval, FloatInterval, MathError, MathLimits,
};

use super::{CellId, Classification, CoverBudget, NativeGridProfile, native_reference};
use crate::geodesic::{PreparedGeodesic, PreparedGeodesicPoint};
use crate::{
    ExecutionPolicy, GeoError, LonLat, Metres, MetricContext, Rat, numerical::geo_math_error,
};

/// The frozen classifier description entering the cover law identity.
pub(super) const CLASSIFIER: &[u8] = b"cube-disk-v2;footprint=cap(u,max-corner-angle)+2^-59;u=normalized-chart-midpoint;M=RN-binary64-midpoint-degrees-of-u;M-cap=cap+latitude-width+longitude-width;sphere-bound=[m*theta,R*theta];metric-bound=binary64-enclosure(d(C,M))+-R*cap;metric-when=2R*cap<sphere-band;outward-binary64;closed-disk;strict-min-max;completed-four-siblings-to-min;sorted-disjoint;reported-pad=1um+threshold-half-ulp;v2";

/// Bound once per invocation: the centre's normal and prepared geodesic data.
pub(super) struct Disk {
    whole: bool,
    math: CoordinateMath,
    pi: FloatInterval,
    centre: [FloatInterval; 3],
    radius: FloatInterval,
    lower_metric: FloatInterval,
    upper_metric: FloatInterval,
    geodesic: &'static PreparedGeodesic,
    source: PreparedGeodesicPoint,
    context: MetricContext,
}

/// Fixed internal numerical allowance of the classifier's binary64 kernels.
/// Every per-cell evaluation is a bounded straight-line computation; this is
/// not caller admission and enters no identity.
const CLASSIFIER_WORK: u64 = u64::MAX >> 2;
const CLASSIFIER_WORKSPACE: usize = 1 << 20;

/// Scratch reserved for one classification: the interval temporaries and the
/// two small exact coordinates of `M`.
pub(super) const CELL_SCRATCH_BYTES: u64 = 4096;

impl Disk {
    pub(super) fn new(
        profile: NativeGridProfile,
        centre: &LonLat,
        radius: &Metres,
        budget: &mut CoverBudget<'_>,
    ) -> Result<Self, GeoError> {
        let reference = native_reference(profile);
        let policy = ExecutionPolicy::geometry();
        // Admit the caller's exact sources before their outward conversion:
        // two directed binary64 roundings of each original rational.
        // Each conversion shifts the shorter operand to the longer one's width
        // and divides for a 54-bit quotient: linear in the operand widths.
        let mut conversion = ExactArithmeticCost::for_operation(ExactOperation::Linear, 1, 1)
            .ok_or(GeoError::ArithmeticOverflow("cover source admission"))?;
        for value in [radius.exact(), centre.longitude(), centre.latitude()] {
            let width = value
                .numerator()
                .bit_len()
                .max(value.denominator().bit_len())
                .saturating_add(64);
            let one = ExactArithmeticCost::for_operation(ExactOperation::Linear, width, 8)
                .ok_or(GeoError::ArithmeticOverflow("cover source admission"))?;
            conversion = conversion
                .followed_by(one)
                .ok_or(GeoError::ArithmeticOverflow("cover source admission"))?;
        }
        budget.exact(conversion, || Ok(()))?;
        // No shortest distance on a native ellipsoid reaches 2^31 m, so a
        // radius beyond it contains the whole surface; it needs no conversion.
        let whole = budget.rational(
            ExactOperation::RationalCompare,
            &[radius.exact()],
            1,
            || Ok(radius.exact() > &Rat::from_i64(1_i64 << 31)),
        )?;
        let mut math = CoordinateMath::new(MathLimits {
            precision_bits: 96,
            max_work: CLASSIFIER_WORK,
            max_workspace_bytes: CLASSIFIER_WORKSPACE,
        })
        .map_err(|error| geo_math_error(&error, policy))?;
        let tables = math
            .prepare_binary64()
            .map_err(|error| geo_math_error(&error, policy))?;
        let pi = FixedInterval::pi(&mut math)
            .and_then(|pi| pi.to_binary64(&mut math))
            .map_err(|error| geo_math_error(&error, policy))?;
        let ellipsoid = reference.ellipsoid();
        let (lower_metric, upper_metric) = ellipsoid.normal_metric_bounds();
        let to_interval = |value: &Rat, math: &mut CoordinateMath| {
            FloatInterval::from_ratio(
                &BigInt::from(value.numerator().clone()),
                &BigInt::from(value.denominator().clone()),
                math,
            )
            .map_err(|error| geo_math_error(&error, policy))
        };
        let radius_interval = if whole {
            FloatInterval::point(f64::from_bits((1023 + 31) << 52))
                .map_err(|error| geo_math_error(&error, policy))?
        } else {
            to_interval(radius.exact(), &mut math)?
        };
        let lower_metric = to_interval(lower_metric.exact(), &mut math)?;
        let upper_metric = to_interval(upper_metric.exact(), &mut math)?;
        let longitude = to_interval(centre.longitude(), &mut math)?;
        let latitude = to_interval(centre.latitude(), &mut math)?;
        let centre_normal = degree_normal(longitude, latitude, pi, &mut math)
            .map_err(|error| geo_math_error(&error, policy))?;
        let geodesic = crate::geodesic::native_prepared(&reference)?;
        let mut context = MetricContext::new(reference, policy)?;
        context.set_prepared_arithmetic(tables);
        let source = geodesic.prepare_point(centre, &mut context)?;
        budget.reserve(source.retained_workspace_bytes())?;
        Ok(Self {
            whole,
            math,
            pi,
            centre: centre_normal,
            radius: radius_interval,
            lower_metric,
            upper_metric,
            geodesic,
            source,
            context,
        })
    }

    pub(super) fn classify(
        &mut self,
        cell: CellId,
        budget: &mut CoverBudget<'_>,
    ) -> Result<Classification, GeoError> {
        if self.whole {
            return Ok(Classification::Inside);
        }
        budget.scratch(CELL_SCRATCH_BYTES)?;
        let policy = ExecutionPolicy::geometry();
        let footprint = match self.footprint(cell) {
            Ok(footprint) => footprint,
            // An undecidable enclosure keeps the cell: it can only refine.
            Err(MathError::PrecisionExhausted | MathError::Domain(_)) => {
                return Ok(Classification::Straddling);
            }
            Err(error) => return Err(geo_math_error(&error, policy)),
        };
        // The validated chunk ends with this block, before the geodesic
        // proof enters its own.
        let sphere = {
            let chunk = self
                .math
                .enter_chunk()
                .map_err(|error| geo_math_error(&error, policy))?;
            self.sphere_status(&footprint, &chunk)
        };
        match sphere {
            Ok(Ok(status)) => return Ok(status),
            Ok(Err(band)) if !band.metric_is_tighter => return Ok(Classification::Straddling),
            Ok(Err(_)) => {}
            Err(MathError::PrecisionExhausted | MathError::Domain(_)) => {
                return Ok(Classification::Straddling);
            }
            Err(error) => return Err(geo_math_error(&error, policy)),
        }
        let Some((lower, upper)) =
            self.geodesic
                .binary64_enclosure(&self.source, &footprint.point, &mut self.context)?
        else {
            return Ok(Classification::Straddling);
        };
        let chunk = self
            .math
            .enter_chunk()
            .map_err(|error| geo_math_error(&error, policy))?;
        match self.metric_status(&footprint, lower, upper, &chunk) {
            Ok(status) => Ok(status),
            Err(MathError::PrecisionExhausted | MathError::Domain(_)) => {
                Ok(Classification::Straddling)
            }
            Err(error) => Err(geo_math_error(&error, policy)),
        }
    }

    /// The chart midpoint direction `u`, the exact interior point `M` and their
    /// complete footprint caps.
    fn footprint(&mut self, cell: CellId) -> Result<Footprint, MathError> {
        let corners = corner_vectors(cell)?;
        let middle = middle_vector(cell)?;
        let chunk = self.math.enter_chunk()?;
        let middle = normalize(&middle, &chunk)?;
        let mut cap = 0.0_f64;
        for corner in corners {
            let corner = normalize(&corner, &chunk)?;
            cap = cap.max(angle(&middle, &corner, self.pi, &chunk)?.upper());
        }
        // The complete physical footprint adds the assignment guard 2^-59.
        let guard = FloatInterval::point(f64::from_bits((1023 - 59) << 52))?;
        let cap = FloatInterval::point(cap)?.add(guard, &chunk)?;
        let horizontal = middle[0]
            .square(&chunk)?
            .add(middle[1].square(&chunk)?, &chunk)?
            .sqrt(&chunk)?;
        let degrees = FloatInterval::point(180.0)?.div(self.pi, &chunk)?;
        // M is the binary64 midpoint of the enclosed latitude and longitude of
        // u, so the great-circle angle from u to M is at most the widths of
        // those two radian enclosures; cos(latitude) <= 1 bounds the longitude.
        let (longitude, latitude, offset) = if horizontal.upper() == 0.0 {
            (0.0, if middle[2].lower() > 0.0 { 90.0 } else { -90.0 }, 0.0)
        } else {
            let latitude = FloatInterval::atan2(middle[2], horizontal, &mut self.math, &chunk)?;
            let (longitude, longitude_width) =
                match FloatInterval::atan2(middle[1], middle[0], &mut self.math, &chunk) {
                    Ok(value) => (
                        midpoint(value.mul(degrees, &chunk)?, &chunk),
                        width(value, &chunk)?,
                    ),
                    // A vertical axis has every longitude; M then sits on the
                    // zero meridian and its full longitude freedom is charged.
                    Err(MathError::PrecisionExhausted) => (0.0, self.pi.upper()),
                    Err(error) => return Err(error),
                };
            let offset = width(latitude, &chunk)?;
            let offset = FloatInterval::point(offset)?
                .add(FloatInterval::point(longitude_width)?, &chunk)?
                .upper();
            (
                longitude,
                midpoint(latitude.mul(degrees, &chunk)?, &chunk),
                offset,
            )
        };
        let point_cap = cap.add(FloatInterval::point(offset)?, &chunk)?;
        let longitude = longitude.clamp(-180.0, 180.0);
        let latitude = latitude.clamp(-90.0, 90.0);
        let point = LonLat::new(
            Rat::from_binary64(longitude).ok_or(MathError::Domain("cover axis longitude"))?,
            Rat::from_binary64(latitude).ok_or(MathError::Domain("cover axis latitude"))?,
        )
        .map_err(|_| MathError::Domain("cover axis outside the geographic domain"))?;
        Ok(Footprint {
            normal: middle,
            cap,
            point,
            point_cap,
        })
    }

    /// The normal-sphere classification, or the band it leaves undecided.
    fn sphere_status(
        &self,
        footprint: &Footprint,
        chunk: &CheckedBinary64,
    ) -> Result<Result<Classification, SphereBand>, MathError> {
        let separation = angle(&self.centre, &footprint.normal, self.pi, chunk)?;
        let chord = chord(&self.centre, &footprint.normal, chunk)?;
        let near = FloatInterval::point(chord.lower())?
            .sub(footprint.cap, chunk)?
            .lower()
            .max(0.0);
        let far = separation
            .add(footprint.cap, chunk)?
            .upper()
            .min(self.pi.upper());
        let lower = self.lower_metric.mul(FloatInterval::point(near)?, chunk)?;
        let upper = self.upper_metric.mul(FloatInterval::point(far)?, chunk)?;
        if lower.lower() > self.radius.upper() {
            return Ok(Ok(Classification::Outside));
        }
        if upper.upper() <= self.radius.lower() {
            return Ok(Ok(Classification::Inside));
        }
        let metric_band = self
            .upper_metric
            .mul(footprint.point_cap, chunk)?
            .mul(FloatInterval::point(2.0)?, chunk)?;
        let sphere_band = upper.sub(lower, chunk)?;
        Ok(Err(SphereBand {
            metric_is_tighter: metric_band.upper() < sphere_band.lower(),
        }))
    }

    /// The classification from a certified enclosure of `d(C, M)`.
    fn metric_status(
        &self,
        footprint: &Footprint,
        lower: f64,
        upper: f64,
        chunk: &CheckedBinary64,
    ) -> Result<Classification, MathError> {
        let spread = self.upper_metric.mul(footprint.point_cap, chunk)?;
        let distance = FloatInterval::from_bounds(lower, upper)?;
        let nearest = distance.sub(spread, chunk)?;
        let farthest = distance.add(spread, chunk)?;
        Ok(if nearest.lower() > self.radius.upper() {
            Classification::Outside
        } else if farthest.upper() <= self.radius.lower() {
            Classification::Inside
        } else {
            Classification::Straddling
        })
    }
}

struct Footprint {
    normal: [FloatInterval; 3],
    cap: FloatInterval,
    point: LonLat,
    point_cap: FloatInterval,
}

struct SphereBand {
    metric_is_tighter: bool,
}

fn width(value: FloatInterval, chunk: &CheckedBinary64) -> Result<f64, MathError> {
    Ok(FloatInterval::point(value.upper())?
        .sub(FloatInterval::point(value.lower())?, chunk)?
        .upper())
}

fn midpoint(value: FloatInterval, chunk: &CheckedBinary64) -> f64 {
    let ops = chunk.ops();
    ops.add(ops.mul(value.lower(), 0.5), ops.mul(value.upper(), 0.5))
        .clamp(value.lower(), value.upper())
}

/// The geodetic unit normal of degree coordinates.
fn degree_normal(
    longitude: FloatInterval,
    latitude: FloatInterval,
    pi: FloatInterval,
    math: &mut CoordinateMath,
) -> Result<[FloatInterval; 3], MathError> {
    let chunk = math.enter_chunk()?;
    let radians = pi.div(FloatInterval::point(180.0)?, &chunk)?;
    let longitude = longitude.mul(radians, &chunk)?;
    let latitude = latitude.mul(radians, &chunk)?;
    let (sin_lon, cos_lon) = longitude.sin_cos(math, &chunk)?;
    let (sin_lat, cos_lat) = latitude.sin_cos(math, &chunk)?;
    Ok([
        cos_lat.mul(cos_lon, &chunk)?,
        cos_lat.mul(sin_lon, &chunk)?,
        sin_lat,
    ])
}

fn normalize(
    vector: &[FloatInterval; 3],
    chunk: &CheckedBinary64,
) -> Result<[FloatInterval; 3], MathError> {
    let norm = vector[0]
        .square(chunk)?
        .add(vector[1].square(chunk)?, chunk)?
        .add(vector[2].square(chunk)?, chunk)?
        .sqrt(chunk)?;
    Ok([
        vector[0].div(norm, chunk)?,
        vector[1].div(norm, chunk)?,
        vector[2].div(norm, chunk)?,
    ])
}

fn chord(
    a: &[FloatInterval; 3],
    b: &[FloatInterval; 3],
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    let mut sum = FloatInterval::point(0.0)?;
    for (a, b) in a.iter().zip(b) {
        sum = sum.add(a.sub(*b, chunk)?.square(chunk)?, chunk)?;
    }
    sum.sqrt(chunk)
}

/// Enclose the angle between unit vectors from their chord `c`: `c <= theta`
/// and, for `0 <= c <= 2`, `theta = 2 asin(c/2) <= c + c^3/4` (see the cell law).
fn angle(
    a: &[FloatInterval; 3],
    b: &[FloatInterval; 3],
    pi: FloatInterval,
    chunk: &CheckedBinary64,
) -> Result<FloatInterval, MathError> {
    let chord = chord(a, b, chunk)?;
    let high = FloatInterval::point(chord.upper().min(2.0))?;
    let cubic = high
        .mul(high, chunk)?
        .mul(high, chunk)?
        .div(FloatInterval::point(4.0)?, chunk)?;
    let upper = high.add(cubic, chunk)?.upper().min(pi.upper());
    FloatInterval::from_bounds(chord.lower().min(upper), upper)
}

/// One face-chart vector component written over the common warp denominator.
fn integer(value: i128) -> Result<FloatInterval, MathError> {
    // Conversion to the nearest binary64; inexact conversions are widened.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let rounded = value as f64;
    #[allow(clippy::cast_possible_truncation)]
    if rounded as i128 == value {
        return FloatInterval::point(rounded);
    }
    FloatInterval::from_bounds(rounded.next_down(), rounded.next_up())
}

fn chart_vector(face: u8, d: i128, u: i128, v: i128) -> Result<[FloatInterval; 3], MathError> {
    let raw = match face {
        0 => [d, u, v],
        1 => [-u, d, v],
        2 => [-u, -v, d],
        3 => [-d, -v, -u],
        4 => [v, -d, -u],
        5 => [v, u, -d],
        _ => return Err(MathError::Domain("invalid cube face")),
    };
    Ok([integer(raw[0])?, integer(raw[1])?, integer(raw[2])?])
}

fn chart_numerators(cell: CellId) -> ([i128; 4], i128) {
    let (mut i, mut j) = super::coordinates(cell);
    if cell.face() & 1 == 1 {
        core::mem::swap(&mut i, &mut j);
    }
    let n = 1_u64 << cell.level();
    let numerators = [i, i + 1, j, j + 1].map(|k| super::super::warp_numerator(k, n));
    let denominator =
        i128::try_from(super::super::warp_denominator(n)).expect("bounded cube denominator");
    (numerators, denominator)
}

fn corner_vectors(cell: CellId) -> Result<[[FloatInterval; 3]; 4], MathError> {
    let ([u0, u1, v0, v1], d) = chart_numerators(cell);
    Ok([
        chart_vector(cell.face(), d, u0, v0)?,
        chart_vector(cell.face(), d, u0, v1)?,
        chart_vector(cell.face(), d, u1, v0)?,
        chart_vector(cell.face(), d, u1, v1)?,
    ])
}

fn middle_vector(cell: CellId) -> Result<[FloatInterval; 3], MathError> {
    let ([u0, u1, v0, v1], d) = chart_numerators(cell);
    chart_vector(cell.face(), 2 * d, u0 + u1, v0 + v1)
}
