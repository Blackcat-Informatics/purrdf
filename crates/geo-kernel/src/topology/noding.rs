// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One exact segment noder for planar relations and identified chart boundaries.

use super::segment::{
    SegmentIntersection, cmp_xy, intersect, intersect_admitted, on_segment, on_segment_admitted,
};
use crate::context::WorkProgress;
use crate::numerical::ExactAdmission;
use crate::{Coord, GeoError, MetricContext, Rat};
use purrdf_xsd::integer::ExactOperation;
use std::borrow::Cow;

trait NodingWork {
    type Error;
    fn visit(&mut self, work: u64, point: Option<&Coord>) -> Result<(), Self::Error>;
    fn exact<T>(
        &mut self,
        operation: ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce() -> T,
    ) -> Result<T, Self::Error>;
    fn intersect(
        &mut self,
        a: &Coord,
        b: &Coord,
        c: &Coord,
        d: &Coord,
    ) -> Result<SegmentIntersection, Self::Error>;
    fn on_segment(&mut self, p: &Coord, a: &Coord, b: &Coord) -> Result<bool, Self::Error>;
}

struct LegacyWork<'callback, F>(&'callback mut F);
impl<E, F: FnMut(u64, Option<&Coord>) -> Result<(), E>> NodingWork for LegacyWork<'_, F> {
    type Error = E;
    fn visit(&mut self, work: u64, point: Option<&Coord>) -> Result<(), E> {
        (self.0)(work, point)
    }
    /// Preserve the legacy coordinate-event callback's error protocol while the
    /// original exact body runs once; the admitted worker separately meters it.
    fn exact<T>(
        &mut self,
        _: ExactOperation,
        _: &[&Rat],
        _: u64,
        evaluate: impl FnOnce() -> T,
    ) -> Result<T, E> {
        Ok(evaluate())
    }
    fn intersect(
        &mut self,
        a: &Coord,
        b: &Coord,
        c: &Coord,
        d: &Coord,
    ) -> Result<SegmentIntersection, E> {
        Ok(intersect(a, b, c, d))
    }
    fn on_segment(&mut self, p: &Coord, a: &Coord, b: &Coord) -> Result<bool, E> {
        Ok(on_segment(p, a, b))
    }
}

struct AdmittedWork<'context, 'observer> {
    context: &'context mut MetricContext,
    progress: &'context mut WorkProgress<'observer>,
    retained: &'context mut u64,
}
impl NodingWork for AdmittedWork<'_, '_> {
    type Error = GeoError;
    fn visit(&mut self, work: u64, point: Option<&Coord>) -> Result<(), GeoError> {
        self.context.charge_work(work)?;
        if let Some(point) = point {
            let bytes = (size_of::<Coord>()
                + point.x().allocated_bytes()
                + point.y().allocated_bytes()) as u64;
            let bytes = bytes
                .checked_mul(2)
                .and_then(|bytes| bytes.checked_add(64))
                .ok_or(GeoError::ArithmeticOverflow("noding event storage"))?;
            self.context.admit_workspace(bytes)?;
            *self.retained = self
                .retained
                .checked_add(bytes)
                .ok_or(GeoError::ArithmeticOverflow("noding retained events"))?;
        }
        self.progress.context_poll(self.context)
    }
    fn exact<T>(
        &mut self,
        operation: ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce() -> T,
    ) -> Result<T, GeoError> {
        ExactAdmission::new(self.context, self.progress).rational(
            operation,
            operands,
            count,
            || Ok(evaluate()),
        )
    }
    fn intersect(
        &mut self,
        a: &Coord,
        b: &Coord,
        c: &Coord,
        d: &Coord,
    ) -> Result<SegmentIntersection, GeoError> {
        intersect_admitted(
            a,
            b,
            c,
            d,
            &mut ExactAdmission::new(self.context, self.progress),
        )
    }
    fn on_segment(&mut self, p: &Coord, a: &Coord, b: &Coord) -> Result<bool, GeoError> {
        on_segment_admitted(
            p,
            a,
            b,
            &mut ExactAdmission::new(self.context, self.progress),
        )
    }
}

pub(crate) fn events<E>(
    edges: &[(&Coord, &Coord)],
    seeds: impl Iterator<Item = Coord>,
    visit: &mut impl FnMut(u64, Option<&Coord>) -> Result<(), E>,
) -> Result<Vec<Coord>, E> {
    events_inner(edges, seeds.map(Cow::Owned), &mut LegacyWork(visit))
}

pub(crate) fn events_admitted<'source>(
    edges: &[(&Coord, &Coord)],
    seeds: impl Iterator<Item = &'source Coord>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Vec<Coord>, GeoError> {
    events_inner(
        edges,
        seeds.map(Cow::Borrowed),
        &mut AdmittedWork {
            context,
            progress,
            retained,
        },
    )
}

fn events_inner<'source, W: NodingWork>(
    edges: &[(&Coord, &Coord)],
    seeds: impl Iterator<Item = Cow<'source, Coord>>,
    work: &mut W,
) -> Result<Vec<Coord>, W::Error> {
    let mut points = Vec::new();
    for point in seeds {
        work.visit(1, None)?;
        work.visit(0, Some(&point))?;
        let point = match point {
            Cow::Owned(point) => point,
            Cow::Borrowed(point) => {
                work.exact(ExactOperation::Linear, &[point.x(), point.y()], 2, || {
                    point.clone()
                })?
            }
        };
        points.push(point);
    }
    for (index, &(a, b)) in edges.iter().enumerate() {
        for &(c, d) in &edges[index + 1..] {
            work.visit(1, None)?;
            match work.intersect(a, b, c, d)? {
                SegmentIntersection::None => {}
                SegmentIntersection::Point(point) => {
                    work.visit(0, Some(&point))?;
                    points.push(point);
                }
                SegmentIntersection::Collinear { from, to } => {
                    work.visit(0, Some(&from))?;
                    points.push(from);
                    work.visit(0, Some(&to))?;
                    points.push(to);
                }
            }
        }
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut points, |a, b| {
        work.visit(1, None)?;
        work.exact(
            ExactOperation::RationalCompare,
            &[a.x(), a.y(), b.x(), b.y()],
            2,
            || cmp_xy(a, b),
        )
    })?;
    let mut unique = 0;
    for index in 0..points.len() {
        let equal = if unique == 0 {
            false
        } else {
            let a = &points[unique - 1];
            let b = &points[index];
            work.exact(
                ExactOperation::RationalCompare,
                &[a.x(), a.y(), b.x(), b.y()],
                2,
                || cmp_xy(a, b).is_eq(),
            )?
        };
        if !equal {
            points.swap(unique, index);
            unique += 1;
        }
    }
    points.truncate(unique);
    Ok(points)
}

pub(crate) fn fragments<'a, E>(
    (start, end): (&Coord, &Coord),
    events: &'a [Coord],
    visit: &mut impl FnMut(u64, Option<&Coord>) -> Result<(), E>,
) -> Result<Vec<(&'a Coord, &'a Coord)>, E> {
    fragments_inner((start, end), events, &mut LegacyWork(visit))
}

pub(crate) fn fragments_admitted<'a>(
    edge: (&Coord, &Coord),
    events: &'a [Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Vec<(&'a Coord, &'a Coord)>, GeoError> {
    fragments_inner(
        edge,
        events,
        &mut AdmittedWork {
            context,
            progress,
            retained,
        },
    )
}

fn fragments_inner<'a, W: NodingWork>(
    (start, end): (&Coord, &Coord),
    events: &'a [Coord],
    work: &mut W,
) -> Result<Vec<(&'a Coord, &'a Coord)>, W::Error> {
    if work.exact(
        ExactOperation::RationalCompare,
        &[start.x(), start.y(), end.x(), end.y()],
        2,
        || start.same_planar(end),
    )? {
        return Ok(Vec::new());
    }
    // Lexicographic order is strictly monotone along every nonzero line:
    // use x when it varies, otherwise y. Reverse it exactly when the original
    // endpoints descend. This is the same order as the former dot-product
    // parameter, without constructing a new rational for every incident event.
    let ascending = work.exact(
        ExactOperation::RationalCompare,
        &[start.x(), start.y(), end.x(), end.y()],
        2,
        || cmp_xy(start, end).is_lt(),
    )?;
    let mut along = Vec::new();
    for point in events {
        work.visit(1, None)?;
        if work.on_segment(point, start, end)? {
            along.push(point);
        }
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut along, |a, b| {
        work.visit(1, None)?;
        work.exact(
            ExactOperation::RationalCompare,
            &[a.x(), a.y(), b.x(), b.y()],
            2,
            || {
                let order = cmp_xy(a, b);
                if ascending { order } else { order.reverse() }
            },
        )
    })?;
    Ok(along.windows(2).map(|pair| (pair[0], pair[1])).collect())
}
