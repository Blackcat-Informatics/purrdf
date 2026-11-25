// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conservative planar candidate pruning for the metered pair walk.

use super::GeoEntry;
use crate::context::WorkProgress;
use crate::geom::{GeometryBody, LeafCoords};
use crate::{
    GeoError, Geometry, GeometryLiteral, IntersectionMatrix, MetricContext, Rat, SpatialRelation,
};

#[derive(Clone, Copy)]
pub(super) struct EnvelopeKeys([i64; 4]);

#[derive(Default)]
pub(super) enum Scan {
    #[default]
    Disabled,
    Observe,
    CheckCrs,
    Homogeneous {
        left: Option<(usize, Option<EnvelopeKeys>)>,
    },
}

impl Scan {
    pub(super) fn observe(&mut self, matrix: &IntersectionMatrix) {
        // When all exact pairs make contact there is no broad-phase win.
        // Wait for a no-contact matrix before spending additional work. This
        // relation examines only matrix cells, not either argument dimension.
        if matches!(self, Self::Observe) && !SpatialRelation::SfIntersects.holds(matrix, 0, 0) {
            *self = Self::CheckCrs;
        }
    }

    pub(super) fn separated(
        &mut self,
        entries: &[GeoEntry],
        left: usize,
        right: usize,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<bool, GeoError> {
        if left == right || matches!(self, Self::Disabled | Self::Observe) {
            return Ok(false);
        }
        let (Some(left_geometry), Some(right_geometry)) = (
            simple_geometry(&entries[left]),
            simple_geometry(&entries[right]),
        ) else {
            return Ok(false);
        };
        if matches!(self, Self::CheckCrs) {
            *self = if homogeneous(entries, context, progress)? {
                Self::Homogeneous { left: None }
            } else {
                Self::Disabled
            };
        }
        let Self::Homogeneous { left: cached } = self else {
            return Ok(false);
        };
        let a = if let Some((position, value)) = *cached
            && position == left
        {
            value
        } else {
            let value = keys(left_geometry, context, progress)?;
            *cached = Some((left, value));
            value
        };
        let Some(a) = a else {
            return Ok(false);
        };
        let Some(b) = keys(right_geometry, context, progress)? else {
            return Ok(false);
        };
        context.charge_work(1)?;
        progress.context_poll(context)?;
        Ok(a.separated(&b))
    }
}

impl EnvelopeKeys {
    // Distinct integer floors preserve strict order. A shared floor never
    // establishes separation, including when the original bounds touch.
    fn separated(&self, other: &Self) -> bool {
        let [west, south, east, north] = self.0;
        let [other_west, other_south, other_east, other_north] = other.0;
        east < other_west || other_east < west || north < other_south || other_north < south
    }
}

pub(super) const fn requires_contact(relation: SpatialRelation) -> bool {
    use SpatialRelation::{
        EhContains, EhCoveredBy, EhCovers, EhDisjoint, EhEquals, EhInside, EhMeet, EhOverlap,
        Rcc8Dc, Rcc8Ec, Rcc8Eq, Rcc8Ntpp, Rcc8Ntppi, Rcc8Po, Rcc8Tpp, Rcc8Tppi, SfContains,
        SfCrosses, SfDisjoint, SfEquals, SfIntersects, SfOverlaps, SfTouches, SfWithin,
    };
    match relation {
        SfDisjoint | EhDisjoint | Rcc8Dc => false,
        SfEquals | SfIntersects | SfTouches | SfCrosses | SfWithin | SfContains | SfOverlaps
        | EhEquals | EhMeet | EhOverlap | EhCovers | EhCoveredBy | EhInside | EhContains
        | Rcc8Eq | Rcc8Ec | Rcc8Po | Rcc8Tppi | Rcc8Tpp | Rcc8Ntpp | Rcc8Ntppi => true,
    }
}

fn homogeneous(
    entries: &[GeoEntry],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    // A witness in a different coordinate system must not be removed by a
    // planar bound on these bytes. Keep the exact visitation order when mixed.
    let mut systems = entries
        .iter()
        .flat_map(|entry| entry.geometries().iter().map(GeometryLiteral::crs));
    let Some(first) = systems.next() else {
        return Ok(false);
    };
    for system in systems {
        context.charge_work(first.as_str().len().min(system.as_str().len()) as u64 + 1)?;
        progress.context_poll(context)?;
        if first != system {
            return Ok(false);
        }
    }
    Ok(true)
}

fn floor(value: &Rat) -> Option<i64> {
    let [numerator, denominator] = value.integer_operands();
    // A canonical denominator is positive. Quotient overflow is impossible
    // even at the most negative numerator because the divisor cannot be -1.
    Some(numerator.to_i64()?.div_euclid(denominator.to_i64()?))
}

fn simple_geometry(entry: &GeoEntry) -> Option<&Geometry> {
    // Alternate literals can span unrelated regions, yielding no useful
    // exclusion from their union. Leave their original pair work unchanged.
    let [literal] = entry.geometries() else {
        return None;
    };
    let geometry = literal.geometry();
    // Multipart and collection walks can hide runs of empty members from
    // coordinate callbacks. Every ring of a simple Polygon has positions.
    matches!(
        geometry.body(),
        GeometryBody::Point(_) | GeometryBody::LineString(_) | GeometryBody::Polygon(_)
    )
    .then_some(geometry)
}

fn keys(
    // Only geometry selected by simple_geometry reaches this bounded iterator.
    geometry: &Geometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<EnvelopeKeys>, GeoError> {
    context.charge_work(1)?;
    progress.context_poll(context)?;
    let mut bounds: Option<[i64; 4]> = None;
    for coord in LeafCoords::of(geometry) {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        let (Some(x), Some(y)) = (floor(coord.x()), floor(coord.y())) else {
            return Ok(None);
        };
        if let Some([west, south, east, north]) = &mut bounds {
            *west = (*west).min(x);
            *south = (*south).min(y);
            *east = (*east).max(x);
            *north = (*north).max(y);
        } else {
            bounds = Some([x, y, x, y]);
        }
    }
    Ok(bounds.map(EnvelopeKeys))
}

#[cfg(test)]
mod tests {
    use super::{GeoEntry, Scan};
    use crate::context::WorkProgress;
    use crate::{
        Coord, CoordDim, Crs, ExecutionPolicy, GeoError, Geometry, GeometryBody, GeometryLiteral,
        MetricContext, MetricWorkObserver, Rat,
    };
    use purrdf_core::TermValue;

    #[test]
    fn an_active_scan_polls_before_excluding_a_pair() {
        struct Stop;
        impl MetricWorkObserver for Stop {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }

        let crs = Crs::new("http://example.org/reference/planar").unwrap();
        let entry = |iri, x| GeoEntry {
            subject: TermValue::iri(iri),
            geometries: vec![GeometryLiteral::new(
                crs.clone(),
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::Point(Some(Coord::new(
                        Rat::from_i64(x),
                        Rat::zero(),
                        None,
                        None,
                    ))),
                )
                .unwrap(),
            )],
        };
        let entries = [
            entry("http://example.org/left", 0),
            entry("http://example.org/right", 4),
        ];
        let matrix = crate::topology::relate(
            entries[0].geometries()[0].geometry(),
            entries[1].geometries()[0].geometry(),
        );
        let mut scan = Scan::Observe;
        scan.observe(&matrix);
        let mut context = MetricContext::from_reference(
            crate::binding::standard_reference(),
            ExecutionPolicy::geometry(),
        )
        .unwrap();
        assert!(matches!(
            scan.separated(
                &entries,
                0,
                1,
                &mut context,
                &mut WorkProgress::integer(Some(&mut Stop)),
            ),
            Err(GeoError::Cancelled)
        ));

        let mut scan = Scan::Observe;
        scan.observe(&matrix);
        assert!(
            scan.separated(
                &entries,
                0,
                1,
                &mut context,
                &mut WorkProgress::integer(None),
            )
            .unwrap()
        );
    }
}
