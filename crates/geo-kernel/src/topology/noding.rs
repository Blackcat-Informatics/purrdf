// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One exact segment noder for planar relations and identified chart boundaries.

use super::segment::{
    SegmentIntersection, intersect, intersect_admitted, on_segment, on_segment_admitted,
};
use crate::context::{RetainedBuilder, WorkProgress};
use crate::numerical::ExactAdmission;
use crate::{Coord, GeoError, MetricContext, Rat};
use core::ops::{Deref, DerefMut};
use purrdf_xsd::integer::ExactOperation;
use std::borrow::Cow;

pub(crate) trait NodingWork {
    type Error;
    type Array<T>: Default + Deref<Target = [T]> + DerefMut + IntoIterator<Item = T>;
    fn push<T>(&mut self, values: &mut Self::Array<T>, value: T) -> Result<(), Self::Error>;
    fn truncate<T>(&mut self, values: &mut Self::Array<T>, length: usize);
    fn release<T>(&mut self, values: Self::Array<T>) -> Result<(), Self::Error>;
    fn append<T>(
        &mut self,
        destination: &mut Self::Array<T>,
        source: Self::Array<T>,
    ) -> Result<(), Self::Error>;
    fn persistent(&mut self, point: Coord) -> Result<Coord, Self::Error>;
    fn visit(&mut self, work: u64, point: Option<&Coord>) -> Result<(), Self::Error>;
    fn compare(&mut self, a: &Rat, b: &Rat) -> Result<core::cmp::Ordering, Self::Error>;
    fn compare_coordinate(
        &mut self,
        a: &Coord,
        b: &Coord,
    ) -> Result<core::cmp::Ordering, Self::Error>;
    fn exact<T>(
        &mut self,
        operation: ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce(
            &crate::numerical::ExactValues<'_>,
        ) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
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

pub(crate) struct LegacyWork<'callback, F>(pub(crate) &'callback mut F);
impl<E, F: FnMut(u64, Option<&Coord>) -> Result<(), E>> NodingWork for LegacyWork<'_, F> {
    type Error = E;
    type Array<T> = Vec<T>;
    fn push<T>(&mut self, values: &mut Vec<T>, value: T) -> Result<(), E> {
        values.push(value);
        Ok(())
    }
    fn truncate<T>(&mut self, values: &mut Vec<T>, length: usize) {
        values.truncate(length);
    }
    fn release<T>(&mut self, values: Vec<T>) -> Result<(), E> {
        drop(values);
        Ok(())
    }
    fn append<T>(&mut self, destination: &mut Vec<T>, mut source: Vec<T>) -> Result<(), E> {
        destination.append(&mut source);
        Ok(())
    }
    fn persistent(&mut self, point: Coord) -> Result<Coord, E> {
        Ok(point)
    }
    fn visit(&mut self, work: u64, point: Option<&Coord>) -> Result<(), E> {
        (self.0)(work, point)
    }
    fn compare(&mut self, a: &Rat, b: &Rat) -> Result<core::cmp::Ordering, E> {
        Ok(a.cmp(b))
    }
    fn compare_coordinate(&mut self, a: &Coord, b: &Coord) -> Result<core::cmp::Ordering, E> {
        Ok(super::segment::cmp_xy(a, b))
    }
    /// Preserve the legacy coordinate-event callback's error protocol while the
    /// original exact body runs once; the admitted worker separately meters it.
    fn exact<T>(
        &mut self,
        _: ExactOperation,
        _: &[&Rat],
        _: u64,
        evaluate: impl FnOnce(
            &crate::numerical::ExactValues<'_>,
        ) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
    ) -> Result<T, E> {
        Ok(evaluate(&crate::numerical::ExactValues::resident())
            .expect("resident noding arithmetic has its infallible allocation contract"))
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

pub(crate) struct AdmittedWork<'context, 'observer> {
    pub(crate) context: &'context mut MetricContext,
    pub(crate) progress: &'context mut WorkProgress<'observer>,
    pub(crate) retained: &'context mut u64,
}
impl NodingWork for AdmittedWork<'_, '_> {
    type Error = GeoError;
    type Array<T> = RetainedBuilder<T>;
    fn push<T>(&mut self, values: &mut RetainedBuilder<T>, value: T) -> Result<(), GeoError> {
        values.push_admitted(value, self.context, self.progress, self.retained)
    }
    fn truncate<T>(&mut self, values: &mut RetainedBuilder<T>, length: usize) {
        values.truncate(length);
    }
    fn release<T>(&mut self, values: RetainedBuilder<T>) -> Result<(), GeoError> {
        values.release_admitted(self.context, self.retained)
    }
    fn append<T>(
        &mut self,
        destination: &mut RetainedBuilder<T>,
        source: RetainedBuilder<T>,
    ) -> Result<(), GeoError> {
        destination.append_admitted(source, self.context, self.progress, self.retained)
    }
    fn persistent(&mut self, point: Coord) -> Result<Coord, GeoError> {
        crate::context::persistent_numeric(point, self.context, self.progress, self.retained)
    }
    fn visit(&mut self, work: u64, _: Option<&Coord>) -> Result<(), GeoError> {
        self.context.charge_work(work)?;
        self.progress.context_poll(self.context)
    }
    fn compare(&mut self, a: &Rat, b: &Rat) -> Result<core::cmp::Ordering, GeoError> {
        ExactAdmission::new(self.context, self.progress).compare(a, b)
    }
    fn compare_coordinate(
        &mut self,
        a: &Coord,
        b: &Coord,
    ) -> Result<core::cmp::Ordering, GeoError> {
        super::segment::compare_inner(
            a,
            b,
            Some(&mut ExactAdmission::new(self.context, self.progress)),
        )
    }
    fn exact<T>(
        &mut self,
        operation: ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce(
            &crate::numerical::ExactValues<'_>,
        ) -> Result<T, purrdf_xsd::integer::LimbScratchError>,
    ) -> Result<T, GeoError> {
        ExactAdmission::new(self.context, self.progress)
            .expression(operation, operands, count, evaluate)
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

pub(crate) fn events_admitted<'source>(
    edges: &[(&Coord, &Coord)],
    seeds: impl Iterator<Item = &'source Coord>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<RetainedBuilder<Coord>, GeoError> {
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

pub(crate) fn events_inner<'source, W: NodingWork>(
    edges: &[(&Coord, &Coord)],
    seeds: impl Iterator<Item = Cow<'source, Coord>>,
    work: &mut W,
) -> Result<W::Array<Coord>, W::Error> {
    let mut points = W::Array::default();
    for point in seeds {
        work.visit(1, None)?;
        work.visit(0, Some(&point))?;
        let point = match point {
            Cow::Owned(point) => point,
            Cow::Borrowed(point) => work.exact(
                ExactOperation::Linear,
                &[point.x(), point.y()],
                2,
                |values| values.planar_coordinate(point),
            )?,
        };
        let point = work.persistent(point)?;
        work.push(&mut points, point)?;
    }
    for (index, &(a, b)) in edges.iter().enumerate() {
        for &(c, d) in &edges[index + 1..] {
            work.visit(1, None)?;
            match work.intersect(a, b, c, d)? {
                SegmentIntersection::None => {}
                SegmentIntersection::Point(point) => {
                    work.visit(0, Some(&point))?;
                    let point = work.persistent(point)?;
                    work.push(&mut points, point)?;
                }
                SegmentIntersection::Collinear { from, to } => {
                    work.visit(0, Some(&from))?;
                    let from = work.persistent(from)?;
                    work.push(&mut points, from)?;
                    work.visit(0, Some(&to))?;
                    let to = work.persistent(to)?;
                    work.push(&mut points, to)?;
                }
            }
        }
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut points, |a, b| {
        work.visit(1, None)?;
        work.compare_coordinate(a, b)
    })?;
    let unique = purrdf_lex::walk::try_partition_unique_by(&mut points, |a, b| {
        Ok(work.compare_coordinate(a, b)?.is_eq())
    })?;
    work.truncate(&mut points, unique);
    Ok(points)
}

pub(crate) fn fragments_admitted<'a>(
    edge: (&Coord, &Coord),
    events: &'a [Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<RetainedBuilder<(&'a Coord, &'a Coord)>, GeoError> {
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

pub(crate) fn fragments_inner<'a, W: NodingWork>(
    (start, end): (&Coord, &Coord),
    events: &'a [Coord],
    work: &mut W,
) -> Result<W::Array<(&'a Coord, &'a Coord)>, W::Error> {
    if work.compare_coordinate(start, end)?.is_eq() {
        return Ok(W::Array::default());
    }
    // Lexicographic order is strictly monotone along every nonzero line:
    // use x when it varies, otherwise y. Reverse it exactly when the original
    // endpoints descend. This is the same order as the former dot-product
    // parameter, without constructing a new rational for every incident event.
    let ascending = work.compare_coordinate(start, end)?.is_lt();
    let mut along = W::Array::default();
    for point in events {
        work.visit(1, None)?;
        if work.on_segment(point, start, end)? {
            work.push(&mut along, point)?;
        }
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut along, |a, b| {
        work.visit(1, None)?;
        let order = work.compare_coordinate(a, b)?;
        Ok(if ascending { order } else { order.reverse() })
    })?;
    let mut fragments = W::Array::default();
    for pair in along.windows(2) {
        work.push(&mut fragments, (pair[0], pair[1]))?;
    }
    work.release(along)?;
    Ok(fragments)
}
