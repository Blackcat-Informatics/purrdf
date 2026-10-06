// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One original chart-cap equation, with exact and certified interval adapters.

use super::{ChartBounds, Classification, ClosedBox, CoverBudget, Footprint, axis_guard, pi_lower};
use crate::context::WorkProgress;
use crate::numerical::{carrier_integer, exact_bounds, fixed_from_rat};
use crate::{GeoError, LonLat, Rat};
use core::cmp::Ordering;
use purrdf_xsd::ieee::ratio::Rounding;
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

type Pairs<T> = Vec<(T, T)>;

pub(in crate::cells) struct Bounds<T> {
    pub(in crate::cells) south: T,
    pub(in crate::cells) north: T,
    pub(in crate::cells) longitudes: Vec<(T, T)>,
}

pub(in crate::cells) struct ChartEnclosure {
    pub(in crate::cells) bounds: Bounds<(Rat, Rat)>,
    pub(in crate::cells) retained: u64,
}

pub(in crate::cells) trait Arithmetic {
    type Number;
    type Error;
    fn original(&mut self, value: &Rat) -> Result<Self::Number, Self::Error>;
    fn add(&mut self, a: &Self::Number, b: &Self::Number) -> Result<Self::Number, Self::Error>;
    fn subtract(&mut self, a: &Self::Number, b: &Self::Number)
    -> Result<Self::Number, Self::Error>;
    fn multiply(&mut self, a: &Self::Number, b: &Self::Number)
    -> Result<Self::Number, Self::Error>;
    fn divide(&mut self, a: &Self::Number, b: &Self::Number) -> Result<Self::Number, Self::Error>;
    fn absolute(&mut self, a: &Self::Number) -> Result<Self::Number, Self::Error>;
    fn minimum(&mut self, a: Self::Number, b: Self::Number) -> Result<Self::Number, Self::Error>;
    fn maximum(&mut self, a: Self::Number, b: Self::Number) -> Result<Self::Number, Self::Error>;
    fn compare(&mut self, a: &Self::Number, b: &Self::Number) -> Result<Ordering, Self::Error>;
}

fn equations<A: Arithmetic>(
    radius: &Rat,
    axis: &LonLat,
    arithmetic: &mut A,
) -> Result<Bounds<A::Number>, A::Error> {
    let guard = arithmetic.original(&axis_guard())?;
    let radius = arithmetic.original(radius)?;
    let radius = arithmetic.add(&radius, &guard)?;
    let turn = arithmetic.original(&Rat::from_i64(180))?;
    let radians = arithmetic.multiply(&radius, &turn)?;
    let pi = arithmetic.original(&pi_lower())?;
    let degrees = arithmetic.divide(&radians, &pi)?;
    let latitude = arithmetic.original(axis.latitude())?;
    let south = arithmetic.subtract(&latitude, &degrees)?;
    let north = arithmetic.add(&latitude, &degrees)?;
    let south_pole = arithmetic.original(&Rat::from_i64(-90))?;
    let north_pole = arithmetic.original(&Rat::from_i64(90))?;
    let south = arithmetic.maximum(south, south_pole)?;
    let north = arithmetic.minimum(north, north_pole)?;
    let south_pole = arithmetic.original(&Rat::from_i64(-90))?;
    let north_pole = arithmetic.original(&Rat::from_i64(90))?;
    let longitudes = if arithmetic.compare(&south, &south_pole)?.is_eq()
        || arithmetic.compare(&north, &north_pole)?.is_eq()
    {
        vec![(
            arithmetic.original(&Rat::from_i64(-180))?,
            arithmetic.original(&Rat::from_i64(180))?,
        )]
    } else {
        let south_abs = arithmetic.absolute(&south)?;
        let north_abs = arithmetic.absolute(&north)?;
        let maximum = arithmetic.maximum(south_abs, north_abs)?;
        let ratio = arithmetic.divide(&maximum, &north_pole)?;
        let one = arithmetic.original(&Rat::one())?;
        let cosine_lower = arithmetic.subtract(&one, &ratio)?;
        let half = arithmetic.divide(&degrees, &cosine_lower)?;
        if !arithmetic.compare(&half, &turn)?.is_lt() {
            vec![(
                arithmetic.original(&Rat::from_i64(-180))?,
                arithmetic.original(&Rat::from_i64(180))?,
            )]
        } else {
            longitude_band(axis.longitude(), &half, arithmetic)?
        }
    };
    Ok(Bounds {
        south,
        north,
        longitudes,
    })
}

fn longitude_band<A: Arithmetic>(
    center: &Rat,
    half: &A::Number,
    arithmetic: &mut A,
) -> Result<Pairs<A::Number>, A::Error> {
    let center = arithmetic.original(center)?;
    let low = arithmetic.subtract(&center, half)?;
    let high = arithmetic.add(&center, half)?;
    let west = arithmetic.original(&Rat::from_i64(-180))?;
    let east = arithmetic.original(&Rat::from_i64(180))?;
    let turn = arithmetic.original(&Rat::from_i64(360))?;
    Ok(if arithmetic.compare(&low, &west)?.is_lt() {
        let wrapped = arithmetic.add(&low, &turn)?;
        vec![(west, high), (wrapped, east)]
    } else if arithmetic.compare(&high, &east)?.is_gt() {
        let wrapped = arithmetic.subtract(&high, &turn)?;
        vec![(west, wrapped), (low, east)]
    } else {
        vec![(low, high)]
    })
}

#[cfg(test)]
struct Exact<'a, 'observer>(&'a mut CoverBudget<'observer>);
#[cfg(test)]
impl Arithmetic for Exact<'_, '_> {
    type Number = Rat;
    type Error = GeoError;
    fn original(&mut self, value: &Rat) -> Result<Rat, GeoError> {
        self.0
            .rational(ExactOperation::Linear, &[value], 1, || Ok(value.clone()))
    }
    fn add(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        self.0
            .rational(ExactOperation::RationalAdd, &[a, b], 1, || Ok(a.add(b)))
    }
    fn subtract(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        self.0
            .rational(ExactOperation::RationalAdd, &[a, b], 1, || Ok(a.sub(b)))
    }
    fn multiply(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        self.0.rational(
            ExactOperation::RationalMultiply,
            &[a, b],
            1,
            || Ok(a.mul(b)),
        )
    }
    fn divide(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        self.0
            .rational(ExactOperation::RationalDivide, &[a, b], 1, || {
                Ok(a.div(b).expect("positive chart divisor"))
            })
    }
    fn absolute(&mut self, a: &Rat) -> Result<Rat, GeoError> {
        self.0
            .rational(ExactOperation::Linear, &[a], 1, || Ok(a.abs()))
    }
    fn minimum(&mut self, a: Rat, b: Rat) -> Result<Rat, GeoError> {
        Ok(if self.0.compare(&a, &b)?.is_lt() {
            a
        } else {
            b
        })
    }
    fn maximum(&mut self, a: Rat, b: Rat) -> Result<Rat, GeoError> {
        Ok(if self.0.compare(&a, &b)?.is_gt() {
            a
        } else {
            b
        })
    }
    fn compare(&mut self, a: &Rat, b: &Rat) -> Result<Ordering, GeoError> {
        self.0.compare(a, b)
    }
}

pub(in crate::cells) struct Interval<'a>(pub(in crate::cells) &'a mut CoordinateMath);
impl Arithmetic for Interval<'_> {
    type Number = FixedInterval;
    type Error = MathError;
    fn original(&mut self, value: &Rat) -> Result<FixedInterval, MathError> {
        fixed_from_rat(value, self.0)
    }
    fn add(&mut self, a: &FixedInterval, b: &FixedInterval) -> Result<FixedInterval, MathError> {
        a.add(b, self.0)
    }
    fn subtract(
        &mut self,
        a: &FixedInterval,
        b: &FixedInterval,
    ) -> Result<FixedInterval, MathError> {
        a.sub(b, self.0)
    }
    fn multiply(
        &mut self,
        a: &FixedInterval,
        b: &FixedInterval,
    ) -> Result<FixedInterval, MathError> {
        a.mul(b, self.0)
    }
    fn divide(&mut self, a: &FixedInterval, b: &FixedInterval) -> Result<FixedInterval, MathError> {
        a.div(b, self.0)
    }
    fn absolute(&mut self, a: &FixedInterval) -> Result<FixedInterval, MathError> {
        a.abs(self.0)
    }
    fn minimum(&mut self, a: FixedInterval, b: FixedInterval) -> Result<FixedInterval, MathError> {
        self.extreme(&a, &b, false)
    }
    fn maximum(&mut self, a: FixedInterval, b: FixedInterval) -> Result<FixedInterval, MathError> {
        self.extreme(&a, &b, true)
    }
    fn compare(&mut self, a: &FixedInterval, b: &FixedInterval) -> Result<Ordering, MathError> {
        self.0.admit_exact(
            4,
            [a.lower(), a.upper(), b.lower(), b.upper()]
                .into_iter()
                .map(purrdf_xsd::BigInt::bits_upper_bound)
                .max()
                .unwrap_or(0),
        )?;
        if a.upper() < b.lower() {
            Ok(Ordering::Less)
        } else if a.lower() > b.upper() {
            Ok(Ordering::Greater)
        } else if a.lower() == a.upper() && a.lower() == b.lower() && b.lower() == b.upper() {
            Ok(Ordering::Equal)
        } else {
            Err(MathError::PrecisionExhausted)
        }
    }
}
impl Interval<'_> {
    fn extreme(
        &mut self,
        a: &FixedInterval,
        b: &FixedInterval,
        maximum: bool,
    ) -> Result<FixedInterval, MathError> {
        self.0.admit_exact(
            4,
            [a.lower(), a.upper(), b.lower(), b.upper()]
                .into_iter()
                .map(purrdf_xsd::BigInt::bits_upper_bound)
                .max()
                .unwrap_or(0),
        )?;
        let lower = if maximum {
            a.lower().max(b.lower())
        } else {
            a.lower().min(b.lower())
        };
        let upper = if maximum {
            a.upper().max(b.upper())
        } else {
            a.upper().min(b.upper())
        };
        FixedInterval::from_bounds(lower.clone(), upper.clone(), self.0)
    }
}

impl Footprint {
    pub(in crate::cells) fn chart_enclosure(
        &self,
        axis: &LonLat,
        precision: u32,
        budget: &mut CoverBudget<'_>,
    ) -> Result<ChartEnclosure, GeoError> {
        let reservation = ExactArithmeticCost::for_operation(
            ExactOperation::Linear,
            u64::from(precision).saturating_add(16),
            16,
        )
        .ok_or(GeoError::ArithmeticOverflow("chart enclosure storage"))?
        .workspace_bytes
        .saturating_mul(4)
        .saturating_add(4096);
        budget.reserve(reservation)?;
        let sources = [&self.radius, axis.longitude(), axis.latitude()];
        let result = budget.math_for_sources_at_precision(precision, &sources, |math, progress| {
            let bounds = equations(&self.radius, axis, &mut Interval(math))?;
            let detach = |value: &FixedInterval,
                          math: &mut CoordinateMath,
                          progress: &mut WorkProgress<'_>|
             -> Result<(Rat, Rat), MathError> {
                let bits = value
                    .lower()
                    .bits_upper_bound()
                    .max(value.upper().bits_upper_bound())
                    .max(value.precision_bits() as usize + 1);
                math.admit_exact_cost(
                    ExactArithmeticCost::for_operation(ExactOperation::Linear, bits as u64, 8)
                        .ok_or(MathError::WorkExhausted)?,
                )?;
                progress.math_poll(math)?;
                Ok(exact_bounds(value))
            };
            let south = detach(&bounds.south, math, progress)?;
            let north = detach(&bounds.north, math, progress)?;
            let mut longitudes = Vec::with_capacity(bounds.longitudes.len());
            for (west, east) in bounds.longitudes {
                longitudes.push((
                    detach(&west, math, progress)?,
                    detach(&east, math, progress)?,
                ));
            }
            Ok(Bounds {
                south,
                north,
                longitudes,
            })
        });
        let retained = result.as_ref().map_or(0, |bounds| {
            (size_of::<ChartEnclosure>() as u64)
                .saturating_add(
                    (bounds.longitudes.capacity() * size_of::<((Rat, Rat), (Rat, Rat))>()) as u64,
                )
                .saturating_add(
                    [
                        &bounds.south.0,
                        &bounds.south.1,
                        &bounds.north.0,
                        &bounds.north.1,
                    ]
                    .into_iter()
                    .chain(
                        bounds
                            .longitudes
                            .iter()
                            .flat_map(|(west, east)| [&west.0, &west.1, &east.0, &east.1]),
                    )
                    .map(|value| value.allocated_bytes() as u64)
                    .sum::<u64>(),
                )
        });
        if retained > reservation {
            budget.release(reservation)?;
            return Err(GeoError::ArithmeticOverflow("chart enclosure storage"));
        }
        budget.release(reservation - retained)?;
        result.map(|bounds| ChartEnclosure { bounds, retained })
    }
    #[cfg(test)]
    pub(in crate::cells) fn chart_bounds(
        &self,
        budget: &mut CoverBudget<'_>,
    ) -> Result<ChartBounds, GeoError> {
        let axis = self.point(budget)?;
        let bounds = equations(&self.radius, &axis, &mut Exact(budget))?;
        Ok(ChartBounds {
            axis,
            south: bounds.south,
            north: bounds.north,
            longitudes: bounds.longitudes,
        })
    }

    /// Prove the identical exact chart walls' directed grid values through
    /// shrinking integer intervals, avoiding intermediate compound fractions.
    pub(in crate::cells) fn chart_bounds_outward(
        &self,
        budget: &mut CoverBudget<'_>,
    ) -> Result<ChartBounds, GeoError> {
        let axis = self.point(budget)?;
        let sources = [&self.radius, axis.longitude(), axis.latitude()];
        let bounds = budget.math_for_sources(&sources, |math, progress| {
            math.reserve_workspace(65_536)?;
            progress.math_poll(math)?;
            let bounds = equations(&self.radius, &axis, &mut Interval(math))?;
            let south = directed(&bounds.south, 15, Rounding::Down, math, progress)?;
            let north = directed(&bounds.north, 15, Rounding::Up, math, progress)?;
            let mut longitudes = Vec::with_capacity(bounds.longitudes.len());
            for (west, east) in bounds.longitudes {
                longitudes.push((
                    directed(&west, 15, Rounding::Down, math, progress)?,
                    directed(&east, 15, Rounding::Up, math, progress)?,
                ));
            }
            Ok(Bounds {
                south,
                north,
                longitudes,
            })
        })?;
        Ok(ChartBounds {
            axis,
            south: bounds.south,
            north: bounds.north,
            longitudes: bounds.longitudes,
        })
    }
}

pub(super) fn classify_box(
    area: &ClosedBox,
    target: &[(Rat, Rat)],
    full_longitude: bool,
    seam: bool,
    footprint: &Footprint,
    budget: &mut CoverBudget<'_>,
) -> Result<Classification, GeoError> {
    let axis = footprint.point(budget)?;
    let mut sources = purrdf_core::SmallVec::<[&Rat; 11]>::new();
    sources.extend([
        &footprint.radius,
        axis.longitude(),
        axis.latitude(),
        &area.south,
        &area.north,
    ]);
    for (west, east) in target {
        sources.extend([west, east]);
    }
    budget.math_for_sources(&sources, |math, _| {
        let mut arithmetic = Interval(math);
        let bounds = equations(&footprint.radius, &axis, &mut arithmetic)?;
        box_equations(area, target, full_longitude, seam, &bounds, &mut arithmetic)
    })
}

fn box_equations<A: Arithmetic>(
    area: &ClosedBox,
    target: &[(Rat, Rat)],
    full_longitude: bool,
    seam: bool,
    bounds: &Bounds<A::Number>,
    arithmetic: &mut A,
) -> Result<Classification, A::Error> {
    let south = arithmetic.original(&area.south)?;
    let north = arithmetic.original(&area.north)?;
    if arithmetic.compare(&bounds.north, &south)?.is_lt()
        || arithmetic.compare(&bounds.south, &north)?.is_gt()
    {
        return Ok(Classification::Outside);
    }
    let latitude_inside = !arithmetic.compare(&bounds.south, &south)?.is_lt()
        && !arithmetic.compare(&bounds.north, &north)?.is_gt();
    if full_longitude {
        return Ok(if latitude_inside {
            Classification::Inside
        } else {
            Classification::Straddling
        });
    }
    let mut intersects = false;
    let mut inside = true;
    for (low, high) in &bounds.longitudes {
        let mut band_inside = false;
        for (west, east) in target {
            let west = arithmetic.original(west)?;
            let east = arithmetic.original(east)?;
            let contact = !arithmetic.compare(high, &west)?.is_lt()
                && !arithmetic.compare(low, &east)?.is_gt();
            let contained = !arithmetic.compare(low, &west)?.is_lt()
                && !arithmetic.compare(high, &east)?.is_gt();
            intersects |= contact;
            band_inside |= contained;
        }
        if seam {
            let west = arithmetic.original(&Rat::from_i64(-180))?;
            let east = arithmetic.original(&Rat::from_i64(180))?;
            intersects |=
                arithmetic.compare(low, &west)?.is_eq() || arithmetic.compare(high, &east)?.is_eq();
        }
        inside &= band_inside;
    }
    Ok(if !intersects {
        Classification::Outside
    } else if latitude_inside && inside {
        Classification::Inside
    } else {
        Classification::Straddling
    })
}

pub(in crate::cells) fn directed(
    value: &FixedInterval,
    places: u32,
    rounding: Rounding,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, MathError> {
    let (lower, upper) = value.round_decimal_with(places, rounding, math)?;
    math.admit_exact(1, lower.bits_upper_bound().max(upper.bits_upper_bound()))?;
    if lower != upper {
        return Err(MathError::PrecisionExhausted);
    }
    let cost = ExactArithmeticCost::decimal_rational(lower.bits_upper_bound() as u64, places)
        .ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(cost)?;
    progress.math_poll(math)?;
    let value = Rat::from_decimal(carrier_integer(&lower), places);
    progress.math_poll(math)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LonLat;
    use crate::cells::{CubeHilbertQ62V1, MixedCoverLimits, NativeGridProfile};

    #[test]
    fn interval_box_decisions_preserve_the_original_exact_classifier() {
        let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
        let limits = MixedCoverLimits {
            max_work_items: 2_000_000_000,
            ..MixedCoverLimits::DEFAULT
        };
        let areas = [
            [-180, -90, 180, 90],
            [170, -15, -170, 15],
            [0, -90, 0, 90],
            [-135, -45, 135, 45],
            [-180, 0, 180, 0],
        ]
        .map(|[west, south, east, north]| {
            ClosedBox::new(
                Rat::from_i64(west),
                Rat::from_i64(south),
                Rat::from_i64(east),
                Rat::from_i64(north),
            )
            .unwrap()
        });
        for level in 0..=2 {
            for face in 0..6 {
                for path in 0..1u64 << (2 * level) {
                    let cell =
                        super::super::CellId::from_path(grid.profile_id(), face, path, level)
                            .unwrap();
                    let footprint = Footprint::new(cell).unwrap();
                    let mut exact_budget = CoverBudget::new_metered(limits, None).unwrap();
                    let axis = footprint.point(&mut exact_budget).unwrap();
                    let bounds =
                        equations(&footprint.radius, &axis, &mut Exact(&mut exact_budget)).unwrap();
                    for area in &areas {
                        let target = area.intervals();
                        let full = area.full_longitude();
                        let seam = area.longitude_contains(&Rat::from_i64(-180));
                        let exact = box_equations(
                            area,
                            &target,
                            full,
                            seam,
                            &bounds,
                            &mut Exact(&mut exact_budget),
                        )
                        .unwrap();
                        let bounded = classify_box(
                            area,
                            &target,
                            full,
                            seam,
                            &footprint,
                            &mut CoverBudget::new_metered(limits, None).unwrap(),
                        )
                        .unwrap();
                        assert_eq!(bounded, exact);
                    }
                }
            }
        }
    }

    #[test]
    fn interval_walls_equal_directed_exact_walls_on_faces_poles_and_seams() {
        let grid = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
        let limits = MixedCoverLimits {
            max_work_items: 2_000_000_000,
            ..MixedCoverLimits::DEFAULT
        };
        let outward =
            |value: &Rat, mode| Rat::from_decimal(value.round_to_scale_with(15, mode), 15);
        for (longitude, latitude) in [
            (0, 0),
            (90, 0),
            (180, 0),
            (-90, 0),
            (0, 90),
            (0, -90),
            (45, 35),
            (135, -35),
            (-135, 35),
            (-45, -35),
            (123, 80),
            (-179, -80),
        ] {
            let point = LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).unwrap();
            for level in [0, 1, 4, 16, 30] {
                let cell = grid.assign(&point, level).unwrap();
                let footprint = Footprint::new(cell).unwrap();
                let exact = footprint
                    .chart_bounds(&mut CoverBudget::new_metered(limits, None).unwrap())
                    .unwrap();
                let bounded = footprint
                    .chart_bounds_outward(&mut CoverBudget::new_metered(limits, None).unwrap())
                    .unwrap();
                assert_eq!(bounded.axis, exact.axis);
                assert_eq!(bounded.south, outward(&exact.south, Rounding::Down));
                assert_eq!(bounded.north, outward(&exact.north, Rounding::Up));
                let expected = exact
                    .longitudes
                    .iter()
                    .map(|(a, b)| (outward(a, Rounding::Down), outward(b, Rounding::Up)))
                    .collect::<Vec<_>>();
                assert_eq!(bounded.longitudes, expected);
            }
        }
    }
}
