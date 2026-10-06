// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Metric preparation retains complete operation images, never export chords.

use super::{GeoProfile, ImageTarget};
use crate::carrier::{reserve_metadata, reserve_metadata_for_push};
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, rational_cost};
use crate::operation::{CoordinateUnit, OperationImageCurve, OperationImagePoint};
use crate::{
    Coord, Crs, ExecutionPolicy, GeoError, GeometryBody, GeometryLiteral, MetricContext,
    MetricWorkObserver, OperationChain, OrientedInterior, PreparedCurve, PreparedEdge,
    PreparedGeometry, PreparedPolygon, PreparedRegion, Rat,
};
use purrdf_xsd::integer::ExactOperation;
use std::sync::Arc;

impl GeoProfile {
    /// Interpret declared axes on an equivalent surface, or prepare the
    /// continuous image under the unique actual registered chain.
    /// Original coordinates, Z/M, epoch and source-linear interpolation remain
    /// immutable inputs to target metrics and topology.
    ///
    /// # Errors
    /// Refuses missing or ambiguous registrations, incompatible targets,
    /// uncertified areal topology and incomplete native resource admission.
    pub fn prepare_transformed_literal(
        &self,
        source: &GeometryLiteral,
        target: &Crs,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
    ) -> Result<PreparedGeometry, GeoError> {
        self.prepare_transformed_inner(source, ImageTarget::Carrier(target), epoch, context, None)
    }

    /// Prepare the original continuous image with bounded governor polling.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::prepare_transformed_literal`].
    pub fn prepare_transformed_literal_metered(
        &self,
        source: &GeometryLiteral,
        target: &Crs,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<PreparedGeometry, GeoError> {
        self.prepare_transformed_inner(
            source,
            ImageTarget::Carrier(target),
            epoch,
            context,
            Some(observer),
        )
    }

    /// Prepare through an explicitly named actual chain, preserving its source law.
    ///
    /// # Errors
    /// Refuses a source-carrier mismatch and the complete native image contract.
    pub fn prepare_transformed_literal_named(
        &self,
        name: &Crs,
        source: &GeometryLiteral,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
    ) -> Result<PreparedGeometry, GeoError> {
        self.prepare_named_inner(name, source, epoch, context, None)
    }

    /// Prepare a named original continuous image with bounded governor polling.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::prepare_transformed_literal_named`].
    pub fn prepare_transformed_literal_named_metered(
        &self,
        name: &Crs,
        source: &GeometryLiteral,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<PreparedGeometry, GeoError> {
        self.prepare_named_inner(name, source, epoch, context, Some(observer))
    }

    /// Prepare one complete source-linear edge through a named actual chain.
    /// Original endpoint axes, Z/M and epoch are retained without materialization.
    ///
    /// # Errors
    /// Refuses an incompatible target, required metadata or complete admission.
    pub fn prepare_transformed_edge_named(
        &self,
        name: &Crs,
        start: &Coord,
        end: &Coord,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
    ) -> Result<PreparedEdge, GeoError> {
        self.prepare_edge_inner(name, start, end, epoch, context, None)
    }

    /// Prepare the same original edge with bounded governor polling.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::prepare_transformed_edge_named`].
    pub fn prepare_transformed_edge_named_metered(
        &self,
        name: &Crs,
        start: &Coord,
        end: &Coord,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<PreparedEdge, GeoError> {
        self.prepare_edge_inner(name, start, end, epoch, context, Some(observer))
    }

    fn prepare_edge_inner(
        &self,
        name: &Crs,
        start: &Coord,
        end: &Coord,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<PreparedEdge, GeoError> {
        let result = (|| {
            context.begin(1)?;
            let mut progress = WorkProgress::new(observer);
            progress.context_poll(context)?;
            let binding = self.operation_with_progress(name, context, &mut progress)?;
            let reference =
                self.reference_with_progress(binding.target(), context, &mut progress)?;
            if !crate::numerical::reference_matches(reference, context, &mut progress)? {
                return Err(GeoError::config(
                    "image edge/context target reference mismatch",
                ));
            }
            let policy = remaining(context)?;
            let admission = {
                let mut nested = progress.nested(
                    context.work_items(),
                    retained(context),
                    context.workspace_peak(),
                );
                crate::prepared::coordinate_source_admission(
                    [start, end],
                    policy.limits(),
                    Some(&mut nested),
                )
            };
            let (_, workspace) = progress.absorb_nested(context, admission)?;
            context.admit_workspace(workspace)?;
            progress.context_poll(context)?;
            let (chain, chain_bytes) = share_chain(binding.chain(), context, &mut progress)?;
            let mut parts = ImageParts {
                owned_retained: chain_bytes,
                ..ImageParts::default()
            };
            let mut preparation = ImagePreparation {
                chain,
                epoch,
                source_swapped: binding.chain().operations()[0].source().swapped_axes,
                work_base: context.work_items(),
                workspace_base: retained(context),
                context,
                progress: &mut progress,
                parts: &mut parts,
            };
            preparation.edge(start, end)
        })();
        finish_image_preparation(result, context)
    }

    fn prepare_named_inner(
        &self,
        name: &Crs,
        source: &GeometryLiteral,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<PreparedGeometry, GeoError> {
        self.prepare_transformed_inner(source, ImageTarget::Named(name), epoch, context, observer)
    }

    fn prepare_transformed_inner(
        &self,
        source: &GeometryLiteral,
        target: ImageTarget<'_>,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<PreparedGeometry, GeoError> {
        let result = (|| {
            context.begin(1)?;
            let baseline = (context.work_items(), retained(context));
            let mut progress = WorkProgress::new(observer);
            progress.context_poll(context)?;
            let (target, selected) =
                self.image_target_with_progress(source.crs(), target, context, &mut progress)?;
            self.prepare_transformed_attempt(
                (source, target),
                epoch,
                context,
                &mut progress,
                selected,
                baseline,
            )
        })();
        finish_image_preparation(result, context)
    }

    fn prepare_transformed_attempt(
        &self,
        (source, target): (&GeometryLiteral, &Crs),
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        selected: Option<&OperationChain>,
        (work_base, workspace_base): (u64, u64),
    ) -> Result<PreparedGeometry, GeoError> {
        let reference = self.reference_with_progress(target, context, progress)?;
        if !crate::numerical::reference_matches(reference, context, progress)? {
            return Err(GeoError::config(
                "transformed preparation/context target reference mismatch",
            ));
        }
        let same_surface = if selected.is_none() {
            match self.reference_with_progress(source.crs(), context, progress) {
                Ok(source) => crate::numerical::reference_same_surface(source, context, progress)?,
                Err(GeoError::UnregisteredCrs(_)) => false,
                Err(error) => return Err(error),
            }
        } else {
            false
        };
        if same_surface {
            let policy = remaining(context)?;
            let result = {
                let mut nested = progress.nested(
                    context.work_items(),
                    retained(context),
                    context.workspace_peak(),
                );
                PreparedGeometry::from_literal_surface_view_metered(
                    source,
                    self,
                    reference,
                    policy,
                    &mut nested,
                )
            };
            let geometry = progress.absorb_nested(context, result)?;
            retain_geometry(&geometry, context, progress)?;
            return Ok(geometry.with_preparation_receipt(
                context.work_items().saturating_sub(work_base),
                retained(context).saturating_sub(workspace_base),
            ));
        }
        let source_reference =
            self.operation_reference_with_progress(source.crs(), context, progress)?;
        let target_reference = self.operation_reference_with_progress(target, context, progress)?;
        let chain = match selected {
            Some(chain) => chain,
            None => {
                self.operation_between_with_progress(source.crs(), target, context, progress)?
            }
        };
        if chain.operations().first().expect("compiled chain").source() != source_reference
            || chain.operations().last().expect("compiled chain").target() != target_reference
            || target_reference.unit != CoordinateUnit::Degrees
        {
            return Err(GeoError::config(
                "transformed preparation requires the registered actual geographic target",
            ));
        }
        let policy = remaining(context)?;
        let result = {
            let mut nested = progress.nested(
                context.work_items(),
                retained(context),
                context.workspace_peak(),
            );
            crate::prepared::source_inventory(source.geometry(), policy.limits(), Some(&mut nested))
        };
        let inventory = progress.absorb_nested(context, result)?;
        context.admit_workspace(inventory.workspace_bytes)?;
        progress.context_poll(context)?;
        let reference_id =
            crate::numerical::reference_identity(context, progress, Some(reference))?;
        let image_identity = chain.image_source_identity(
            inventory.content_id,
            target,
            reference_id.digest(),
            epoch,
            context,
            progress,
        )?;
        if source_reference.unit == CoordinateUnit::Degrees {
            for coordinate in source.geometry().coords() {
                ExactAdmission::new(context, progress).rational(
                    ExactOperation::RationalCompare,
                    &[coordinate.x(), coordinate.y()],
                    4,
                    || self.lon_lat(source.crs(), coordinate).map(|_| ()),
                )?;
            }
        }
        let (chain, chain_bytes) = share_chain(chain, context, progress)?;
        let mut parts = ImageParts {
            owned_retained: chain_bytes,
            ..ImageParts::default()
        };
        let mut preparation = ImagePreparation {
            chain,
            epoch,
            source_swapped: source_reference.swapped_axes,
            context,
            progress,
            parts: &mut parts,
            work_base,
            workspace_base,
        };
        let mut pending =
            purrdf_lex::walk::WorkList::<&crate::Geometry, 16>::with(source.geometry());
        while let Some(geometry) = pending.pop() {
            preparation.context.charge_work(1)?;
            preparation.progress.context_poll(preparation.context)?;
            match geometry.body() {
                GeometryBody::Point(point) => {
                    if let Some(point) = point {
                        preparation.point(point)?;
                    }
                }
                GeometryBody::MultiPoint(points) => {
                    for point in points.iter().flatten() {
                        preparation.point(point)?;
                    }
                }
                GeometryBody::LineString(line) => preparation.line(line)?,
                GeometryBody::MultiLineString(lines) => {
                    for line in lines {
                        preparation.line(line)?;
                    }
                }
                GeometryBody::Polygon(rings) => preparation.polygon(geometry.dim(), rings)?,
                GeometryBody::MultiPolygon(polygons) => {
                    for rings in polygons {
                        preparation.polygon(geometry.dim(), rings)?;
                    }
                }
                GeometryBody::GeometryCollection(members) => {
                    // The still-live original source inventory reserves 256B
                    // per child before either traversal. This covers both the
                    // old/new doubled reference frontier; its 64KiB entry
                    // allowance covers the inline first sixteen slots.
                    for member in members.iter().rev() {
                        preparation.context.charge_work(1)?;
                        preparation.progress.context_poll(preparation.context)?;
                        pending.push(member);
                    }
                }
            }
        }
        preparation
            .finish()
            .map(|geometry| geometry.with_image_source_identity(image_identity))
    }
}

#[derive(Default)]
struct ImageParts {
    points: Vec<OperationImagePoint>,
    curves: Vec<PreparedCurve>,
    polygons: Vec<PreparedPolygon>,
    owned_retained: u64,
}

struct ImagePreparation<'context, 'epoch, 'observer> {
    chain: Arc<OperationChain>,
    epoch: Option<&'epoch Rat>,
    source_swapped: bool,
    context: &'context mut MetricContext,
    progress: &'context mut WorkProgress<'observer>,
    parts: &'context mut ImageParts,
    work_base: u64,
    workspace_base: u64,
}

impl ImagePreparation<'_, '_, '_> {
    fn copy(&mut self, coordinate: &Coord) -> Result<(Coord, u64), GeoError> {
        self.context
            .charge_work(self.chain.operations().len() as u64)?;
        self.progress.context_poll(self.context)?;
        for operation in self.chain.operations() {
            operation
                .model()
                .validate_presence(coordinate.z(), self.epoch)?;
        }
        let operands = [coordinate.x(), coordinate.y()]
            .into_iter()
            .chain(coordinate.z())
            .chain(coordinate.m())
            .chain(self.epoch)
            .collect::<purrdf_core::SmallVec<[&Rat; 5]>>();
        let cost = rational_cost(ExactOperation::Linear, &operands, operands.len() as u64 * 2)
            .ok_or(GeoError::ArithmeticOverflow(
                "continuous source coordinate copy",
            ))?;
        self.context
            .retain_workspace(cost.workspace_bytes, &mut self.parts.owned_retained)?;
        let coordinate = self
            .progress
            .exact(self.context, cost, || Ok(coordinate.clone()))?;
        Ok((coordinate, cost.workspace_bytes))
    }

    fn point(&mut self, coordinate: &Coord) -> Result<(), GeoError> {
        let (source, copy_bytes) = self.copy(coordinate)?;
        let policy = remaining(self.context)?;
        let result = {
            let mut nested = self.progress.nested(
                self.context.work_items(),
                retained(self.context),
                self.context.workspace_peak(),
            );
            OperationImagePoint::new_metered(
                source,
                Arc::clone(&self.chain),
                self.context.reference().clone(),
                self.epoch.cloned(),
                policy,
                &mut nested,
            )
        };
        let point = self.progress.absorb_nested(self.context, result)?;
        self.context
            .admit_workspace(point.retained_workspace_bytes())?;
        self.parts.owned_retained = self
            .parts
            .owned_retained
            .checked_add(point.retained_workspace_bytes())
            .ok_or(GeoError::ArithmeticOverflow(
                "symbolic point retained storage",
            ))?;
        self.progress.context_poll(self.context)?;
        self.context
            .release_retained_workspace(copy_bytes, &mut self.parts.owned_retained)?;
        self.admit_slots::<OperationImagePoint>(1)?;
        reserve_metadata_for_push(&mut self.parts.points, self.context, self.progress)?;
        self.parts.points.push(point);
        Ok(())
    }

    fn curve(&mut self, coordinates: &[Coord], reverse: bool) -> Result<PreparedCurve, GeoError> {
        self.admit_slots::<PreparedEdge>(coordinates.len().saturating_sub(1))?;
        let mut edges = Vec::new();
        reserve_metadata(
            &mut edges,
            coordinates.len().saturating_sub(1),
            self.context,
            self.progress,
        )?;
        for offset in 0..coordinates.len().saturating_sub(1) {
            let index = if reverse {
                coordinates.len() - 2 - offset
            } else {
                offset
            };
            let (start, end) = if reverse {
                (&coordinates[index + 1], &coordinates[index])
            } else {
                (&coordinates[index], &coordinates[index + 1])
            };
            edges.push(self.edge(start, end)?);
        }
        Ok(PreparedCurve::new(edges))
    }

    fn edge(&mut self, start: &Coord, end: &Coord) -> Result<PreparedEdge, GeoError> {
        let (start, start_bytes) = self.copy(start)?;
        let (end, end_bytes) = self.copy(end)?;
        let policy = remaining(self.context)?;
        let result = {
            let mut nested = self.progress.nested(
                self.context.work_items(),
                retained(self.context),
                self.context.workspace_peak(),
            );
            OperationImageCurve::new_metered(
                start,
                end,
                Arc::clone(&self.chain),
                self.context.reference().clone(),
                self.epoch.cloned(),
                policy,
                &mut nested,
            )
        };
        let image = self.progress.absorb_nested(self.context, result)?;
        self.context
            .admit_workspace(image.retained_workspace_bytes())?;
        self.parts.owned_retained = self
            .parts
            .owned_retained
            .checked_add(image.retained_workspace_bytes())
            .ok_or(GeoError::ArithmeticOverflow("image curve retained storage"))?;
        self.progress.context_poll(self.context)?;
        self.context
            .release_retained_workspace(start_bytes, &mut self.parts.owned_retained)?;
        self.context
            .release_retained_workspace(end_bytes, &mut self.parts.owned_retained)?;
        Ok(PreparedEdge::Transformed(Box::new(image)))
    }

    fn line(&mut self, coordinates: &[Coord]) -> Result<(), GeoError> {
        if !coordinates.is_empty() {
            let curve = self.curve(coordinates, false)?;
            self.admit_slots::<PreparedCurve>(1)?;
            reserve_metadata_for_push(&mut self.parts.curves, self.context, self.progress)?;
            self.parts.curves.push(curve);
        }
        Ok(())
    }

    fn polygon(&mut self, dim: crate::CoordDim, rings: &crate::Rings) -> Result<(), GeoError> {
        if rings.is_empty() {
            return Ok(());
        }
        self.context
            .charge_work(self.chain.operations().len() as u64)?;
        self.progress.context_poll(self.context)?;
        if !self.chain.registered_injective_area() {
            return self.cell_polygon(dim, rings);
        }
        for ring in rings {
            if crate::measure::signed_ring_orientation_admitted(
                ring,
                &mut ExactAdmission::new(self.context, self.progress),
            )? == 0
            {
                return self.cell_polygon(dim, rings);
            }
        }
        self.admit_slots::<crate::atlas::oriented::OrientedRing>(rings.len())?;
        let mut witnesses = Vec::new();
        reserve_metadata(&mut witnesses, rings.len(), self.context, self.progress)?;
        for (index, ring) in rings.iter().enumerate() {
            let orientation = crate::measure::signed_ring_orientation_admitted(
                ring,
                &mut ExactAdmission::new(self.context, self.progress),
            )?;
            let counterclockwise = (orientation > 0) != self.source_swapped;
            let curve = self.curve(ring, counterclockwise != (index == 0))?;
            let mut child = self.context.remaining_child()?;
            let result = {
                let mut nested = self.progress.nested(
                    self.context.work_items(),
                    retained(self.context),
                    self.context.workspace_peak(),
                );
                crate::atlas::oriented::OrientedRing::new_metered(curve, &mut child, &mut nested)
            };
            witnesses.push(
                self.progress
                    .absorb_child_result(self.context, &child, result)?,
            );
        }
        self.admit_slots::<PreparedPolygon>(1)?;
        reserve_metadata_for_push(&mut self.parts.polygons, self.context, self.progress)?;
        self.parts
            .polygons
            .push(PreparedPolygon::from_oriented_rings(
                witnesses,
                OrientedInterior::Left,
                self.context.reference(),
            )?);
        Ok(())
    }

    fn cell_polygon(&mut self, dim: crate::CoordDim, rings: &crate::Rings) -> Result<(), GeoError> {
        if dim != crate::CoordDim::Xy {
            // A carrier polygon declares boundary ordinates, not an interior
            // height/measure field at newly isolated multiple preimages.
            return Err(GeoError::PrecisionExhausted {
                bits: self.context.policy().limits().max_precision_bits,
            });
        }
        // The complete closed source boundary survives even if all open
        // interiors disappear or overlap. It is selected region support, not
        // an explicit input curve with traversal-multiplicity length.
        self.admit_slots::<PreparedCurve>(rings.len())?;
        let mut boundaries = Vec::new();
        reserve_metadata(&mut boundaries, rings.len(), self.context, self.progress)?;
        for ring in rings {
            if ring.len() > 1 {
                boundaries.push(self.curve(ring, false)?);
            }
        }
        let chain = Arc::clone(&self.chain);
        let epoch = self.epoch;
        let source_swapped = self.source_swapped;
        let parts = &mut *self.parts;
        chain.visit_exact_area_cells(
            rings,
            epoch,
            self.context,
            self.progress,
            |mut vertices, certificate, context, progress| {
                let mut preparation = ImagePreparation {
                    chain: Arc::clone(&chain),
                    epoch,
                    source_swapped,
                    context,
                    progress,
                    parts,
                    work_base: 0,
                    workspace_base: 0,
                };
                let (first, first_bytes) = preparation.copy(&vertices[0])?;
                reserve_metadata(&mut vertices, 1, preparation.context, preparation.progress)?;
                vertices.push(first);
                preparation.admit_slots::<PreparedCurve>(1)?;
                preparation.admit_slots::<PreparedPolygon>(1)?;
                reserve_metadata_for_push(
                    &mut preparation.parts.polygons,
                    preparation.context,
                    preparation.progress,
                )?;
                let polygon = match certificate {
                    crate::operation::ExactAreaCell::Region(certificate) => {
                        let reverse = certificate.orientation() < 0;
                        let curve = preparation.curve(&vertices, reverse)?;
                        let witness = crate::atlas::oriented::OrientedRing::from_operation_cell(
                            curve,
                            &certificate,
                            reverse,
                        )?;
                        let polygon = PreparedPolygon::from_oriented_rings(
                            vec![witness],
                            OrientedInterior::Left,
                            preparation.context.reference(),
                        )?;
                        if let Some((bounds, inner, bytes)) = certificate.into_bounds() {
                            preparation
                                .context
                                .retain_workspace(bytes, &mut preparation.parts.owned_retained)?;
                            preparation.progress.context_poll(preparation.context)?;
                            polygon.with_certified_angular_rectangle(bounds, inner, bytes)
                        } else {
                            polygon
                        }
                    }
                    crate::operation::ExactAreaCell::ClosedSupport => {
                        let curve = preparation.curve(&vertices, false)?;
                        PreparedPolygon::from_selected_support(
                            vec![curve],
                            preparation.context.reference(),
                        )?
                    }
                };
                preparation.parts.polygons.push(polygon);
                drop(vertices);
                preparation.context.release_retained_workspace(
                    first_bytes,
                    &mut preparation.parts.owned_retained,
                )?;
                Ok(())
            },
        )?;
        // Certified open faces can decide Interior before testing their
        // absorbed source boundary support. Exterior/contact queries still
        // examine every component through the same closed-set union body.
        if !boundaries.is_empty() {
            self.admit_slots::<PreparedPolygon>(1)?;
            reserve_metadata_for_push(&mut self.parts.polygons, self.context, self.progress)?;
            self.parts
                .polygons
                .push(PreparedPolygon::from_selected_support(
                    boundaries,
                    self.context.reference(),
                )?);
        }
        Ok(())
    }

    fn finish(self) -> Result<PreparedGeometry, GeoError> {
        // Keep original owners admitted throughout the final inventory and
        // immutable-slice publication. Its completed receipt takes over before
        // the old construction allowance is released.
        let parts = core::mem::take(self.parts);
        let policy = remaining(self.context)?;
        let result = {
            let mut nested = self.progress.nested(
                self.context.work_items(),
                retained(self.context),
                self.context.workspace_peak(),
            );
            PreparedGeometry::from_parts_with_symbolic_metered(
                self.context.reference().clone(),
                Vec::new(),
                parts.points,
                parts.curves,
                PreparedRegion::polygons(parts.polygons),
                policy,
                &mut nested,
            )
        };
        let geometry = self.progress.absorb_nested(self.context, result)?;
        retain_geometry(&geometry, self.context, self.progress)?;
        self.context.release_workspace(parts.owned_retained)?;
        Ok(geometry.with_preparation_receipt(
            self.context.work_items().saturating_sub(self.work_base),
            retained(self.context).saturating_sub(self.workspace_base),
        ))
    }

    fn admit_slots<T>(&mut self, count: usize) -> Result<(), GeoError> {
        let count = u64::try_from(count)
            .map_err(|_| GeoError::ArithmeticOverflow("image output metadata count"))?;
        self.context.charge_work(count)?;
        // Four slots per inserted member cover geometric Vec growth, the
        // simultaneous old/new allocation and its final Arc slice conversion.
        // Every original Rat/image owner is retained separately above.
        let bytes = count
            .checked_mul(size_of::<T>() as u64)
            .and_then(|bytes| bytes.checked_mul(4))
            .and_then(|bytes| bytes.checked_add(2 * size_of::<usize>() as u64))
            .ok_or(GeoError::ArithmeticOverflow(
                "image output metadata storage",
            ))?;
        self.context
            .retain_workspace(bytes, &mut self.parts.owned_retained)?;
        self.progress.context_poll(self.context)
    }
}

fn finish_image_preparation<T>(
    result: Result<T, GeoError>,
    context: &mut MetricContext,
) -> Result<T, GeoError> {
    if result.is_ok() {
        context.retain_current_preparation()?;
    } else {
        // Numerical pools prepared during this attempt remain owned by the
        // context. Drop candidate images first, then release only its transient
        // source/output allowances; neither their live baseline nor the first
        // observer refusal is erased by cleanup.
        context.release_transient_workspace()?;
    }
    result
}

fn retained(context: &MetricContext) -> u64 {
    context
        .policy()
        .limits()
        .max_workspace_bytes
        .saturating_sub(context.remaining_workspace())
}

fn share_chain(
    chain: &OperationChain,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Arc<OperationChain>, u64), GeoError> {
    context.charge_work(16)?;
    // The table and compiled models are immutable Arc owners. The original
    // clone copies only fixed-width chain metadata and increments its shared
    // table count; the enclosing Arc is this preparation's sole new owner.
    let bytes = (size_of::<OperationChain>() + 2 * size_of::<usize>()) as u64;
    context.admit_workspace(bytes)?;
    progress.context_poll(context)?;
    Ok((Arc::new(chain.clone()), bytes))
}

fn remaining(context: &MetricContext) -> Result<ExecutionPolicy, GeoError> {
    context
        .policy()
        .remaining_after(context.work_items(), retained(context))
}

fn retain_geometry(
    geometry: &PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    context.admit_workspace(geometry.retained_workspace_bytes())?;
    progress.context_poll(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::OperationModel;
    use crate::{AxisOrder, CoordinateOperation, GeographicReference};
    use purrdf_hash::hex::Digest32;
    use purrdf_iri::vocab::ogc;

    #[derive(Default)]
    struct Receipt {
        work: u64,
        peak: u64,
        calls: u64,
        cancel: bool,
        cancel_after: Option<u64>,
    }
    impl MetricWorkObserver for Receipt {
        fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
            self.calls += 1;
            if self.cancel
                || self
                    .cancel_after
                    .is_some_and(|limit| self.work + work >= limit)
            {
                return Err(GeoError::Cancelled);
            }
            self.work = self.work.checked_add(work).unwrap();
            self.peak = self.peak.checked_add(growth).unwrap();
            Ok(())
        }
    }

    fn mercator_profile() -> (GeoProfile, Crs, Crs) {
        let mut profile = GeoProfile::standard();
        let source = Crs::new("http://example.org/projected").unwrap();
        let target = Crs::new(ogc::CRS84).unwrap();
        let operation = CoordinateOperation::compile(
            crate::operation::OperationReference {
                realization: Digest32::new([7; 32]),
                unit: CoordinateUnit::Metres,
                swapped_axes: false,
            },
            profile.operation_reference(&target).unwrap(),
            OperationModel::MercatorToGeographic {
                radius: Rat::from_i64(6_378_137),
                eccentricity_squared: Rat::zero(),
                square_domain: true,
            },
        )
        .unwrap();
        let name = Crs::new("http://example.org/inverse-projection").unwrap();
        profile
            .register_operation(
                name,
                source.clone(),
                target.clone(),
                OperationChain::compile(vec![operation]).unwrap(),
            )
            .unwrap();
        (profile, source, target)
    }

    fn polynomial_image_profile(
        degree: u16,
        terms: Vec<crate::operation::PolynomialTerm>,
        swapped_axes: bool,
    ) -> (GeoProfile, Crs, Crs) {
        let mut profile = GeoProfile::standard();
        let source = Crs::new("http://example.org/original-polynomial-plane").unwrap();
        let name = Crs::new("http://example.org/complete-polynomial-image").unwrap();
        let target = Crs::new(ogc::CRS84).unwrap();
        let plane = crate::operation::OperationReference {
            realization: Digest32::new([17; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes,
        };
        let intermediate = crate::operation::OperationReference {
            realization: Digest32::new([18; 32]),
            swapped_axes: false,
            ..plane
        };
        let polynomial = crate::operation::Polynomial2d::new(
            degree,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            terms,
        )
        .unwrap();
        let chain = OperationChain::compile(vec![
            CoordinateOperation::compile(
                plane,
                intermediate,
                OperationModel::Polynomial2d(polynomial),
            )
            .unwrap(),
            CoordinateOperation::compile(
                intermediate,
                profile.operation_reference(&target).unwrap(),
                OperationModel::MercatorToGeographic {
                    radius: Rat::one(),
                    eccentricity_squared: Rat::zero(),
                    square_domain: true,
                },
            )
            .unwrap(),
        ])
        .unwrap();
        profile
            .register_operation(name.clone(), source.clone(), target, chain)
            .unwrap();
        (profile, source, name)
    }

    fn fold_terms() -> Vec<crate::operation::PolynomialTerm> {
        vec![
            crate::operation::PolynomialTerm {
                x_power: 2,
                y_power: 0,
                x_coefficient: Rat::one(),
                y_coefficient: Rat::zero(),
            },
            crate::operation::PolynomialTerm {
                x_power: 0,
                y_power: 1,
                x_coefficient: Rat::zero(),
                y_coefficient: Rat::one(),
            },
        ]
    }

    #[test]
    fn exact_folded_areal_image_retains_all_preimages_holes_and_original_identity() {
        let (profile, source, name) = polynomial_image_profile(2, fold_terms(), false);
        let literal = crate::wkt::parse(&format!("<{source}> POLYGON((-0.2 -0.2,0.2 -0.2,0.2 0.2,-0.2 0.2,-0.2 -0.2),(-0.1 -0.1,-0.1 0.1,0.1 0.1,0.1 -0.1,-0.1 -0.1))"), crate::standard_vocabulary().default_wkt_crs()).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let image = profile
            .prepare_transformed_literal_named(&name, &literal, None, &mut context)
            .unwrap();
        assert_eq!(
            image.curves(),
            [],
            "areal image support is not explicit traversal"
        );
        let PreparedRegion::Polygons(polygons) = image.region() else {
            panic!("complete cell union");
        };
        assert!(polygons.iter().any(PreparedPolygon::closed_support));
        assert!(polygons.iter().any(|polygon| !polygon.closed_support()));
        for (longitude, latitude, expected) in [
            ("1", "0", crate::Set::Interior),
            ("0.2", "0", crate::Set::Exterior),
            ("0.2", "8", crate::Set::Interior),
            ("-0.2", "8", crate::Set::Exterior),
            ("0", "8", crate::Set::Boundary),
        ] {
            let point = crate::LonLat::new(
                Rat::parse_decimal(longitude).unwrap(),
                Rat::parse_decimal(latitude).unwrap(),
            )
            .unwrap();
            let mut query = MetricContext::new(
                GeographicReference::wgs84(),
                ExecutionPolicy::new(crate::ExecutionLimits {
                    max_work_items: 4_000_000,
                    ..crate::ExecutionLimits::GEOMETRY
                })
                .unwrap(),
            )
            .unwrap();
            assert_eq!(
                crate::atlas::locate(&point, image.region(), &mut query).unwrap_or_else(|error| {
                    panic!(
                        "image query {longitude},{latitude}: {error:?}; work={}",
                        query.work_items()
                    )
                }),
                expected
            );
        }
        let mut raised = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(crate::ExecutionLimits {
                max_work_items: 4_000_000,
                max_subdivision_levels: 40,
                ..crate::ExecutionLimits::GEOMETRY
            })
            .unwrap(),
        )
        .unwrap();
        let alternate = profile
            .prepare_transformed_literal_named(&name, &literal, None, &mut raised)
            .unwrap();
        assert_eq!(image.id(), alternate.id());
        assert_eq!(image.region(), alternate.region());
    }

    #[test]
    fn exact_rank_lost_areal_image_remains_selected_support() {
        let terms = vec![
            crate::operation::PolynomialTerm {
                x_power: 1,
                y_power: 0,
                x_coefficient: Rat::one(),
                y_coefficient: Rat::zero(),
            },
            crate::operation::PolynomialTerm {
                x_power: 0,
                y_power: 1,
                x_coefficient: Rat::one(),
                y_coefficient: Rat::zero(),
            },
        ];
        let (profile, source, name) = polynomial_image_profile(1, terms, false);
        let literal = crate::wkt::parse(
            &format!("<{source}> POLYGON((0 0,0.1 0,0.1 0.1,0 0.1,0 0))"),
            crate::standard_vocabulary().default_wkt_crs(),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let image = profile
            .prepare_transformed_literal_named(&name, &literal, None, &mut context)
            .unwrap();
        assert_eq!(image.curves(), []);
        let PreparedRegion::Polygons(polygons) = image.region() else {
            panic!("closed image support");
        };
        assert!(polygons.iter().all(PreparedPolygon::closed_support));
        let mut query = MetricContext::wgs84().unwrap();
        let point = crate::LonLat::new(Rat::from_i64(1), Rat::zero()).unwrap();
        let edge = polygons[0].rings()[0].edges().first().unwrap();
        let mut direct = MetricContext::wgs84().unwrap();
        assert!(crate::atlas::point::contact(edge, &point, &mut direct).unwrap());
        assert_eq!(
            crate::atlas::locate(&point, image.region(), &mut query).unwrap(),
            crate::Set::Boundary
        );
        let away =
            crate::LonLat::new(Rat::from_i64(1), Rat::parse_decimal("0.001").unwrap()).unwrap();
        assert_eq!(
            crate::atlas::locate(&away, image.region(), &mut query).unwrap(),
            crate::Set::Exterior
        );
    }

    #[test]
    fn exact_constant_and_degenerate_areal_images_keep_their_closed_support() {
        for (degree, terms, spelling, on) in [
            (0, Vec::new(), "POLYGON((0 0,0.1 0,0.1 0.1,0 0.1,0 0))", "0"),
            (
                1,
                vec![crate::operation::PolynomialTerm {
                    x_power: 1,
                    y_power: 0,
                    x_coefficient: Rat::one(),
                    y_coefficient: Rat::zero(),
                }],
                "POLYGON((0 0,0.1 0,0.05 0,0 0))",
                "1",
            ),
        ] {
            let (profile, source, name) = polynomial_image_profile(degree, terms, false);
            let literal = crate::wkt::parse(
                &format!("<{source}> {spelling}"),
                crate::standard_vocabulary().default_wkt_crs(),
            )
            .unwrap();
            let mut context = MetricContext::wgs84().unwrap();
            let image = profile
                .prepare_transformed_literal_named(&name, &literal, None, &mut context)
                .unwrap();
            assert_eq!(image.curves(), []);
            let point = crate::LonLat::new(Rat::parse_decimal(on).unwrap(), Rat::zero()).unwrap();
            let away = crate::LonLat::new(Rat::parse_decimal(on).unwrap(), Rat::one()).unwrap();
            assert_eq!(
                crate::atlas::locate(&point, image.region(), &mut context).unwrap(),
                crate::Set::Boundary
            );
            assert_eq!(
                crate::atlas::locate(&away, image.region(), &mut context).unwrap(),
                crate::Set::Exterior
            );
        }
    }

    #[test]
    fn exact_area_preparation_retains_complete_output_and_refusal_receipts() {
        let (profile, source, name) = polynomial_image_profile(2, fold_terms(), false);
        let literal = crate::wkt::parse(
            &format!("<{source}> POLYGON((-0.2 -0.1,0.3 -0.1,0.3 0.2,-0.2 0.2,-0.2 -0.1))"),
            crate::standard_vocabulary().default_wkt_crs(),
        )
        .unwrap();
        let mut plain = MetricContext::wgs84().unwrap();
        let expected = profile
            .prepare_transformed_literal_named(&name, &literal, None, &mut plain)
            .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        context.set_preparation_work(17).unwrap();
        context.set_retained_workspace(256).unwrap();
        let mut receipt = Receipt::default();
        let actual = profile
            .prepare_transformed_literal_named_metered(
                &name,
                &literal,
                None,
                &mut context,
                &mut receipt,
            )
            .unwrap();
        assert_eq!(actual.id(), expected.id());
        assert_eq!(actual.region(), expected.region());
        assert_eq!(receipt.work, context.work_items());
        assert_eq!(receipt.peak, context.workspace_peak());
        let retained = context.retained_workspace_bytes();
        assert!(retained >= 256 + actual.retained_workspace_bytes());
        context.begin(1).unwrap();
        assert_eq!(context.retained_workspace_bytes(), retained);
        for stop in [1, plain.work_items() / 3, plain.work_items() * 2 / 3] {
            let mut context = MetricContext::wgs84().unwrap();
            context.set_retained_workspace(256).unwrap();
            let mut receipt = Receipt {
                cancel_after: Some(stop),
                ..Receipt::default()
            };
            let refused = profile.prepare_transformed_literal_named_metered(
                &name,
                &literal,
                None,
                &mut context,
                &mut receipt,
            );
            assert_eq!(refused.unwrap_err(), GeoError::Cancelled);
            if let Some(scratch) = context.integer_scratch() {
                assert_eq!(scratch.available(), scratch.destination_capacity());
            }
            assert_eq!(
                context.current_workspace_bytes(),
                context.retained_workspace_bytes()
            );
            let retained = context.retained_workspace_bytes();
            context.begin(1).unwrap();
            assert_eq!(context.retained_workspace_bytes(), retained);
        }
    }

    #[test]
    fn image_entry_admits_large_names_and_mismatch_errors_before_copying() {
        let (profile, source, name) = polynomial_image_profile(2, fold_terms(), false);
        let long = Crs::new(format!("http://example.org/{}", "x".repeat(65_536))).unwrap();
        let literals = [&source, &long].map(|crs| {
            crate::wkt::parse(
                &format!("<{crs}> POINT(0 0)"),
                crate::standard_vocabulary().default_wkt_crs(),
            )
            .unwrap()
        });
        let point = Coord::xy(Rat::zero(), Rat::zero());
        for entry in 0..8 {
            let policy = ExecutionPolicy::new(crate::ExecutionLimits {
                max_work_items: 1024,
                ..crate::ExecutionLimits::GEOMETRY
            })
            .unwrap();
            let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let refused = match entry {
                0 => profile
                    .prepare_transformed_literal_named(&long, &literals[0], None, &mut context)
                    .map(|_| ()),
                1 => profile
                    .prepare_transformed_literal_named(&name, &literals[1], None, &mut context)
                    .map(|_| ()),
                2 => profile
                    .prepare_transformed_literal(&literals[0], &long, None, &mut context)
                    .map(|_| ()),
                3 => profile
                    .prepare_transformed_edge_named(&long, &point, &point, None, &mut context)
                    .map(|_| ()),
                4 => profile
                    .transform_literal_named(&long, &literals[0], None, &mut context)
                    .map(|_| ()),
                5 => profile
                    .transform_literal_named(&name, &literals[1], None, &mut context)
                    .map(|_| ()),
                6 => profile
                    .transform_literal(&literals[0], &long, None, &mut context)
                    .map(|_| ()),
                _ => profile.validate_literal_in_context(&literals[1], &mut context),
            };
            let allocations = window.close();
            assert!(matches!(
                refused,
                Err(GeoError::WorkExhausted { limit: 1024 })
            ));
            assert_eq!(allocations.allocations, 0, "image entry {entry}");
            assert_eq!(allocations.requested_bytes, 0, "image entry {entry}");
            assert_eq!(context.retained_workspace_bytes(), 0);
        }
    }

    #[test]
    fn folded_image_metrics_and_cover_match_independent_inverse_projection_polygon() {
        let (mut profile, source, name) = polynomial_image_profile(2, fold_terms(), false);
        let control_source = Crs::new("http://example.org/independent-image-plane").unwrap();
        let control_name = Crs::new("http://example.org/independent-inverse-projection").unwrap();
        let inverse = profile.operation(&name).unwrap().chain().operations()[1].clone();
        profile
            .register_operation(
                control_name.clone(),
                control_source.clone(),
                Crs::new(ogc::CRS84).unwrap(),
                OperationChain::compile(vec![inverse]).unwrap(),
            )
            .unwrap();
        let originals = [
            (
                &source,
                &name,
                "POLYGON((-0.2 -0.2,0.2 -0.2,0.2 0.2,-0.2 0.2,-0.2 -0.2),(-0.1 -0.1,-0.1 0.1,0.1 0.1,0.1 -0.1,-0.1 -0.1))",
            ),
            (
                &control_source,
                &control_name,
                "POLYGON((0 -0.2,0.04 -0.2,0.04 0.2,0 0.2,0 0.1,0.01 0.1,0.01 -0.1,0 -0.1,0 -0.2))",
            ),
        ];
        let policy = ExecutionPolicy::new(crate::ExecutionLimits {
            max_work_items: 100_000_000,
            ..crate::ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let images = originals.map(|(source, name, spelling)| {
            let literal = crate::wkt::parse(
                &format!("<{source}> {spelling}"),
                crate::standard_vocabulary().default_wkt_crs(),
            )
            .unwrap();
            let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
            profile
                .prepare_transformed_literal_named(name, &literal, None, &mut context)
                .unwrap()
        });
        for (metric_index, measure) in [
            crate::ellipsoidal::length,
            crate::ellipsoidal::perimeter,
            crate::ellipsoidal::area,
        ]
        .into_iter()
        .enumerate()
        {
            let mut image_index = 0;
            let estimates = images.each_ref().map(|image| {
                let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
                let estimate = measure(image, &mut context).unwrap_or_else(|error| {
                    panic!(
                        "image metric {metric_index}/{image_index}: {error:?}; work={}",
                        context.work_items()
                    )
                });
                image_index += 1;
                estimate
            });
            let difference = estimates[0].exact().sub(estimates[1].exact()).abs();
            assert!(difference <= estimates[0].error_bound().add(&estimates[1].error_bound()));
        }
        let grid = crate::cells::CubeHilbertQ62V1::new(crate::cells::NativeGridProfile::Wgs84);
        let covers = images.each_ref().map(|image| {
            grid.cover_region_fixed(
                image.region(),
                2,
                crate::cells::FixedCoverLimits {
                    max_work_items: policy.limits().max_work_items,
                    ..crate::cells::FixedCoverLimits::DEFAULT
                },
            )
            .unwrap()
        });
        assert_eq!(covers[0].cells(), covers[1].cells());
    }

    #[test]
    fn swapped_source_axes_preserve_folded_physical_orientation() {
        let images = [false, true].map(|swapped| {
            let (profile, source, name) = polynomial_image_profile(2, fold_terms(), swapped);
            let ring = if swapped {
                "(-0.1 -0.2,-0.1 0.3,0.2 0.3,0.2 -0.2,-0.1 -0.2)"
            } else {
                "(-0.2 -0.1,0.3 -0.1,0.3 0.2,-0.2 0.2,-0.2 -0.1)"
            };
            let literal = crate::wkt::parse(
                &format!("<{source}> POLYGON({ring})"),
                crate::standard_vocabulary().default_wkt_crs(),
            )
            .unwrap();
            profile
                .prepare_transformed_literal_named(
                    &name,
                    &literal,
                    None,
                    &mut MetricContext::wgs84().unwrap(),
                )
                .unwrap()
        });
        for point in [
            crate::LonLat::new(Rat::one(), Rat::zero()).unwrap(),
            crate::LonLat::new(Rat::one(), Rat::from_i64(-8)).unwrap(),
        ] {
            let locations = images.each_ref().map(|image| {
                let mut context = MetricContext::new(
                    GeographicReference::wgs84(),
                    ExecutionPolicy::new(crate::ExecutionLimits {
                        max_work_items: 4_000_000,
                        ..crate::ExecutionLimits::GEOMETRY
                    })
                    .unwrap(),
                )
                .unwrap();
                crate::atlas::locate(&point, image.region(), &mut context).unwrap()
            });
            assert_eq!(locations[0], locations[1]);
            assert_eq!(
                locations[0],
                if point.latitude().is_zero() {
                    crate::Set::Interior
                } else {
                    crate::Set::Exterior
                }
            );
        }
    }

    #[test]
    fn equivalent_surface_view_retains_original_axes_and_ordinates() {
        let mut profile = GeoProfile::standard();
        let epsg = Crs::new(ogc::EPSG4326).unwrap();
        profile
            .register_reference(
                epsg.clone(),
                GeographicReference::wgs84().with_axes(AxisOrder::LatLon),
            )
            .unwrap();
        let literal = crate::wkt::parse(
            &format!("<{epsg}> POINT ZM(49 -123 7 9)"),
            crate::standard_vocabulary().default_wkt_crs(),
        )
        .unwrap();
        let target = Crs::new(ogc::CRS84).unwrap();
        let original = PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let viewed = profile
            .prepare_transformed_literal(&literal, &target, None, &mut context)
            .unwrap();
        assert_eq!(viewed.points()[0].source(), original.points()[0].source());
        assert_eq!(viewed.points()[0].point(), original.points()[0].point());
        assert_eq!(viewed.reference(), &GeographicReference::wgs84());
        assert_ne!(viewed.id(), original.id());
        let unchanged = PreparedGeometry::from_literal_surface_view(
            &literal,
            &profile,
            original.reference(),
            profile.policy(),
        )
        .unwrap();
        assert_eq!(unchanged.id(), original.id());
    }

    #[test]
    fn projected_source_metrics_retain_the_unrounded_continuous_edge() {
        let (profile, source, target) = mercator_profile();
        let literal = crate::wkt::parse(
            &format!("<{source}> LINESTRING ZM(0 0 7 9,1000 0 8 10)"),
            crate::standard_vocabulary().default_wkt_crs(),
        )
        .unwrap();
        assert_eq!(profile.metric_reference(&[&source]).unwrap(), target);
        let mut context = MetricContext::wgs84().unwrap();
        let prepared = profile
            .prepare_transformed_literal(&literal, &target, None, &mut context)
            .unwrap();
        let PreparedEdge::Transformed(image) = &prepared.curves()[0].edges()[0] else {
            panic!("the original operation image must remain the metric edge");
        };
        assert_eq!(*image.source_endpoints().1.x(), Rat::from_i64(1_000));
        assert_eq!(image.source_endpoints().1.m(), Some(&Rat::from_i64(10)));
        let name = Crs::new("http://example.org/inverse-projection").unwrap();
        let mut worker = MetricContext::wgs84().unwrap();
        let PreparedEdge::Transformed(single) = profile
            .prepare_transformed_edge_named(
                &name,
                image.source_endpoints().0,
                image.source_endpoints().1,
                None,
                &mut worker,
            )
            .unwrap()
        else {
            panic!("named edge preparation must preserve the original continuous image");
        };
        assert_eq!(single.id(), image.id());
        assert_eq!(single.source_endpoints(), image.source_endpoints());
        let metric = crate::ellipsoidal::length(&prepared, &mut context).unwrap();
        assert_eq!(*metric.exact(), Rat::from_i64(1_000));
    }

    #[test]
    fn named_image_edge_observes_complete_preparation_and_refuses_before_copying() {
        let (profile, _, _) = mercator_profile();
        let name = Crs::new("http://example.org/inverse-projection").unwrap();
        let start = Coord::xy(Rat::zero(), Rat::zero());
        let end = Coord::xy(Rat::from_i64(1000), Rat::zero());
        let epoch = Rat::parse_decimal("2026.5").unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        context.set_preparation_work(17).unwrap();
        context.set_retained_workspace(256).unwrap();
        let mut receipt = Receipt::default();
        let edge = profile
            .prepare_transformed_edge_named_metered(
                &name,
                &start,
                &end,
                Some(&epoch),
                &mut context,
                &mut receipt,
            )
            .unwrap();
        assert!(matches!(edge, PreparedEdge::Transformed(_)));
        assert_eq!(receipt.work, context.work_items());
        assert_eq!(receipt.peak, context.workspace_peak());
        let baseline = context.retained_workspace_bytes();
        context.begin(1).unwrap();
        assert_eq!(context.retained_workspace_bytes(), baseline);

        for limits in [
            crate::ExecutionLimits {
                max_work_items: 1,
                ..crate::ExecutionLimits::GEOMETRY
            },
            crate::ExecutionLimits {
                max_workspace_bytes: 65_535,
                ..crate::ExecutionLimits::GEOMETRY
            },
        ] {
            let mut limited = MetricContext::new(
                GeographicReference::wgs84(),
                ExecutionPolicy::new(limits).unwrap(),
            )
            .unwrap();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result =
                profile.prepare_transformed_edge_named(&name, &start, &end, None, &mut limited);
            let allocation = window.close();
            assert!(matches!(
                result,
                Err(GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. })
            ));
            assert_eq!(allocation.allocations, 0);
            assert_eq!(allocation.requested_bytes, 0);
            assert_eq!(limited.retained_workspace_bytes(), 0);
        }
        let mut context = MetricContext::wgs84().unwrap();
        let mut receipt = Receipt {
            cancel: true,
            ..Receipt::default()
        };
        let result = profile.prepare_transformed_edge_named_metered(
            &name,
            &start,
            &end,
            None,
            &mut context,
            &mut receipt,
        );
        assert_eq!(result.unwrap_err(), GeoError::Cancelled);
        assert_eq!(receipt.calls, 1);
        assert_eq!(context.retained_workspace_bytes(), 0);
    }

    #[test]
    fn original_transformed_point_is_dimension_zero_and_no_datum_is_inferred() {
        let (mut profile, source, target) = mercator_profile();
        let literal = crate::wkt::parse(
            &format!("<{source}> POINT(1000 0)"),
            crate::standard_vocabulary().default_wkt_crs(),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let prepared = profile
            .prepare_transformed_literal(&literal, &target, None, &mut context)
            .unwrap();
        assert_eq!(prepared.points().len(), 0);
        assert_eq!(prepared.curves().len(), 0);
        assert_eq!(prepared.symbolic_points().len(), 1);
        assert_eq!(
            *prepared.symbolic_points()[0].source().x(),
            Rat::from_i64(1_000)
        );
        let cgcs = Crs::new("http://example.org/cgcs").unwrap();
        profile
            .register_reference(cgcs.clone(), GeographicReference::cgcs2000())
            .unwrap();
        assert!(matches!(
            profile.metric_reference(&[&target, &cgcs]),
            Err(GeoError::MissingOperation { .. })
        ));
    }
}
