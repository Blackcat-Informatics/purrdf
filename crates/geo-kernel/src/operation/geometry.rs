// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete continuous source-linear curve images and certified materialization.
//! For a panel, the image box encloses every original coordinate. A whole-panel
//! Jacobian also bounds f' on that panel; subtracting the endpoint secant gives
//! an error bounded by half the derivative range. No sampled flatness test is a
//! certificate, and discontinuities or unadmitted panels never become chords.

use super::{
    CoordinateUnit, OperationChain, OperationCoordinates, OperationModel, OperationPoint,
    OperationSolverLimits,
};
use crate::carrier::MaterializationStorage;
use crate::context::WorkProgress;
use crate::numerical::{exact_bounds, fixed_from_bounds, fixed_from_rat, geo_math_error};
use crate::{
    Coord, Crs, GeoError, GeographicReference, Geometry, GeometryBody, GeometryLiteral, Metres,
    MetricContext, MetricWorkObserver, Rat, SemanticLawId,
};
use purrdf_hash::{Domain, hex::Digest32};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError, RootJacobian2};
use std::cell::RefCell;
use std::sync::Arc;

mod critical;
pub(crate) use critical::{AreaCellCertificate, ExactAreaCell};
mod curve;
mod directional;
mod point;
pub use curve::{OperationImageCurve, OperationImageEnclosure};
pub use point::OperationImagePoint;

const IMAGE_LAW_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/continuous-image-law/v1");
const IMAGE_SOURCE_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/continuous-image-source/v1");
const IMAGE_DESCRIPTION: &[u8] = b"complete-original-coordinate-linear-image;actual-chain-no-intermediate-quantization;exact-matched-native-TM-roundtrip=whole-source-admitted-nonpolar-positive-real-derivative-identity;fixed96-whole-panel-Jacobian-or-actual-3D-directional-range-or-image-box;secant-bound=outward-fixed96-L1;geographic-L1-factor=ceil-micrometre-original-path-equation;dyadic-original-parameter-panels;secant-target0.099998m;degree15-metre6-half-even;Hausdorff-total0.1m;actual-height-epoch-affine-matrix;preserve-M;folds=exact-labelled-source-strips-convex-dyadic-partition;complete-image-box-diameter0.09m-first;otherwise-whole-box-mean-value-injectivity-I-minus-AJ-infinity-less1-or-separated-polynomial-weak-derivatives-plus-actual-center-nonzero;undefined-differential-after-proved-image=dyadic-subdivide;critical-image-box-outward-grid-guard;complete-boundary-images-and-closed-component-union;v1";

/// One original source pointer and its successful complete admission. Keeping
/// these fields private prevents applying a receipt to another geometry.
pub(crate) struct AdmittedImageSource<'a> {
    source: &'a Geometry,
    inventory: crate::prepared::SourceInventory,
}

impl<'a> AdmittedImageSource<'a> {
    /// Admit the original tree before any geographic validation or image copy.
    pub(crate) fn prepare(
        source: &'a Geometry,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        let policy = context.policy().remaining_after(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
        )?;
        let inventory = {
            let mut nested = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            crate::prepared::source_inventory(source, policy.limits(), Some(&mut nested))
        };
        let inventory = progress.absorb_nested(context, inventory)?;
        context.admit_workspace(inventory.workspace_bytes)?;
        if let Err(error) = progress.context_poll(context) {
            context.release_workspace(inventory.workspace_bytes)?;
            return Err(error);
        }
        Ok(Self { source, inventory })
    }

    pub(crate) const fn workspace_bytes(&self) -> u64 {
        self.inventory.workspace_bytes
    }
}

enum ImageSource<'a> {
    Original(&'a Geometry),
    Prepared(AdmittedImageSource<'a>),
}

/// Explicit physical metric of the target carrier, independent of its IRI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageMetric {
    /// Euclidean easting/northing in metres.
    CartesianMetres,
    /// Ground ellipsoidal longitude/latitude of this actual target reference.
    Geographic(Box<GeographicReference>),
}
impl ImageMetric {
    fn factor(
        &self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Rat, GeoError> {
        match self {
            Self::CartesianMetres => Ok(Rat::one()),
            Self::Geographic(reference) => crate::numerical::quantized_source_linear_bound(
                &crate::LonLat::new(Rat::zero(), Rat::zero()).expect("exact origin"),
                &crate::LonLat::new(Rat::one(), Rat::zero()).expect("one degree"),
                Some(reference),
                context,
                progress,
            ),
        }
    }
}

/// Certified materialization of a complete continuous carrier image.
#[derive(Clone, Debug)]
pub struct GeometryImage {
    literal: Arc<GeometryLiteral>,
    chain: Digest32,
    source: Digest32,
    law: SemanticLawId,
    vertices: u64,
    source_vertices: u64,
    source_nesting_depth: u64,
    output_receipt: crate::MaterializedOutputReceipt,
}
impl PartialEq for GeometryImage {
    fn eq(&self, other: &Self) -> bool {
        self.literal == other.literal
            && self.chain == other.chain
            && self.source == other.source
            && self.law == other.law
            && self.vertices == other.vertices
            && self.source_vertices == other.source_vertices
            && self.source_nesting_depth == other.source_nesting_depth
    }
}
impl Eq for GeometryImage {}
impl GeometryImage {
    /// Frozen complete continuous-image output law, available before execution.
    #[must_use]
    pub fn output_law_id() -> SemanticLawId {
        SemanticLawId::from_digest(crate::profile::hash_fields(
            IMAGE_LAW_DOMAIN,
            [IMAGE_DESCRIPTION],
        ))
    }
    /// Materialized target carrier with its caller-declared actual CRS.
    #[must_use]
    pub fn literal(&self) -> &GeometryLiteral {
        &self.literal
    }
    /// Exact compiled operation chain identity.
    #[must_use]
    pub const fn operation_id(&self) -> Digest32 {
        self.chain
    }
    /// Original exact carrier, operation, target and epoch identity.
    #[must_use]
    pub const fn source_id(&self) -> Digest32 {
        self.source
    }
    /// Frozen completed materialization law, independent of admission limits.
    #[must_use]
    pub const fn law_id(&self) -> SemanticLawId {
        self.law
    }
    crate::metric::source_bound_certificate!(b"complete-image-Hausdorff-bound=0.1m");
    /// Complete Hausdorff bound, including final coordinate quantization.
    #[must_use]
    pub fn error_bound(&self) -> Metres {
        Metres::new(Rat::parse_decimal("0.1").expect("frozen certificate"))
    }
    /// Number of admitted output vertices, including repeated closures.
    #[must_use]
    pub const fn vertices(&self) -> u64 {
        self.vertices
    }
    /// Complete original vertex count and collection nesting depth, useful for
    /// proving full ingestion independently of adaptive output subdivision.
    #[must_use]
    pub const fn source_counts(&self) -> (u64, u64) {
        (self.source_vertices, self.source_nesting_depth)
    }
}

impl OperationChain {
    /// One original continuous-image identity home. Adaptive proof partitions,
    /// output approximations and admission limits are not source content.
    pub(crate) fn image_source_identity(
        &self,
        content: Digest32,
        target: &Crs,
        metric: Digest32,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Digest32, GeoError> {
        let (epoch_fields, bytes) = super::epoch_fields_admitted(epoch, context, progress)?;
        context.admit_workspace(bytes)?;
        let result = (|| {
            let numerator = epoch_fields.first().map_or(&[][..], Vec::as_slice);
            let denominator = epoch_fields.get(1).map_or(&[][..], Vec::as_slice);
            let work = u64::try_from(target.as_str().len())
                .ok()
                .and_then(|work| work.checked_add(numerator.len() as u64))
                .and_then(|work| work.checked_add(denominator.len() as u64))
                .and_then(|work| work.checked_add(32 * 3 + 8 * 6))
                .ok_or(GeoError::ArithmeticOverflow(
                    "continuous image source framing",
                ))?;
            context.charge_work(work)?;
            progress.context_poll(context)?;
            Ok(crate::profile::hash_fields(
                IMAGE_SOURCE_DOMAIN,
                [
                    content.as_bytes().as_slice(),
                    self.id().as_bytes().as_slice(),
                    target.as_str().as_bytes(),
                    metric.as_bytes().as_slice(),
                    numerator,
                    denominator,
                ],
            ))
        })();
        drop(epoch_fields);
        context.release_workspace(bytes)?;
        result
    }
    pub(crate) fn registered_injective_area(&self) -> bool {
        self.operations()
            .iter()
            .all(|operation| operation.model().registered_injective_area())
    }
    /// Materialize every complete source-linear image with a 0.1 metre Hausdorff
    /// certificate. Epoch and target metric are explicit; no axis, height or
    /// datum declaration is inferred from coordinates or a CRS name.
    ///
    /// # Errors
    /// Refuses incompatible units, uncertified areal image topology, crossing a
    /// model domain, insufficient precision, or complete resource admission.
    pub fn materialize_geometry(
        &self,
        source: &Geometry,
        target: Crs,
        metric: &ImageMetric,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
    ) -> Result<GeometryImage, GeoError> {
        self.materialize_inner(
            ImageSource::Original(source),
            target,
            metric,
            epoch,
            context,
            None,
        )
    }
    /// Materialize a complete image with bounded work and cancellation callbacks.
    ///
    /// # Errors
    /// Also preserves an observer's original refusal without a partial carrier.
    pub fn materialize_geometry_metered(
        &self,
        source: &Geometry,
        target: Crs,
        metric: &ImageMetric,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<GeometryImage, GeoError> {
        self.materialize_inner(
            ImageSource::Original(source),
            target,
            metric,
            epoch,
            context,
            Some(observer),
        )
    }

    pub(crate) fn materialize_admitted_image(
        &self,
        source: AdmittedImageSource<'_>,
        target: Crs,
        metric: &ImageMetric,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<GeometryImage, GeoError> {
        self.materialize_inner(
            ImageSource::Prepared(source),
            target,
            metric,
            epoch,
            context,
            observer,
        )
    }

    fn materialize_inner(
        &self,
        source: ImageSource<'_>,
        target: Crs,
        metric: &ImageMetric,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<GeometryImage, GeoError> {
        let unit = self
            .operations()
            .last()
            .expect("compiled nonempty chain")
            .target()
            .unit;
        if (unit == CoordinateUnit::Metres) != matches!(metric, ImageMetric::CartesianMetres) {
            return Err(GeoError::config(
                "image metric contradicts declared target units",
            ));
        }
        context.begin(1)?;
        if context.policy().limits().max_precision_bits < 96 {
            return Err(GeoError::PrecisionExhausted {
                bits: context.policy().limits().max_precision_bits,
            });
        }
        let mut progress = WorkProgress::new(observer);
        progress.context_poll(context)?;
        let metric_id = if let ImageMetric::Geographic(reference) = metric {
            let endpoint = self.operations().last().expect("compiled chain").target();
            let binding =
                crate::numerical::reference_identity(context, &mut progress, Some(reference))?;
            if endpoint.realization != binding.digest()
                || endpoint.swapped_axes != (reference.axes() == crate::AxisOrder::LatLon)
            {
                return Err(GeoError::config(
                    "image metric disagrees with actual target realization or axes",
                ));
            }
            binding.digest()
        } else {
            Digest32::new([0; 32])
        };
        let (source, inventory, source_bytes) = match source {
            ImageSource::Original(source) => {
                let admitted = AdmittedImageSource::prepare(source, context, &mut progress)?;
                let bytes = admitted.inventory.workspace_bytes;
                (admitted.source, admitted.inventory, bytes)
            }
            ImageSource::Prepared(admitted) => {
                crate::PreparedSourceReceipt::new(
                    admitted.inventory.content_id,
                    admitted.inventory.work_items,
                    admitted.inventory.workspace_bytes,
                )
                .validate_context(context)?;
                (admitted.source, admitted.inventory, 0)
            }
        };
        let source_bits = inventory
            .maximum_original_operand_bits()
            .max(self.max_original_operand_bits())
            .max(epoch.map_or(0, crate::numerical::rational_operand_bits));
        if let Err(error) = context.prepare_integer_scratch_for_observed(source_bits, &mut progress)
        {
            context.release_workspace(source_bytes)?;
            return Err(error);
        }
        if let Err(error) = progress.context_poll(context) {
            context.release_workspace(source_bytes)?;
            return Err(error);
        }
        let chain_id = self.id();
        let source_id = match self.image_source_identity(
            inventory.content_id,
            &target,
            metric_id,
            epoch,
            context,
            &mut progress,
        ) {
            Ok(identity) => identity,
            Err(error) => {
                context.release_workspace(source_bytes)?;
                return Err(error);
            }
        };
        let factor = match metric.factor(context, &mut progress) {
            Ok(factor) => factor,
            Err(error) => {
                context.release_workspace(source_bytes)?;
                return Err(error);
            }
        };
        let mut retained = MaterializationStorage::admitted_transient(source_bytes);
        let output_bytes = (size_of::<GeometryImage>() as u64)
            .checked_add((size_of::<GeometryLiteral>() + 2 * size_of::<usize>()) as u64)
            .and_then(|bytes| bytes.checked_add(target.retained_text_bytes() as u64))
            .and_then(|bytes| bytes.checked_add(inventory.output_structure_bytes()))
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized image containers",
            ));
        let admission = output_bytes.and_then(|bytes| retained.admit_output(bytes, context));
        if let Err(error) = admission {
            retained.finish(false, context, &mut progress)?;
            return Err(error);
        }
        let worker = RefCell::new(ImageWorker {
            chain: self,
            factor,
            epoch,
            context,
            progress: &mut progress,
            vertices: 0,
            retained,
            point_cache: purrdf_core::FastMap::default(),
        });
        let result = crate::geom::try_fold(
            source,
            |node| worker.borrow_mut().leaf(node),
            |node, members| {
                let mut worker = worker.borrow_mut();
                worker.context.charge_work(1)?;
                worker.progress_poll()?;
                Geometry::new(node.dim(), GeometryBody::GeometryCollection(members))
            },
        );
        let ImageWorker {
            context,
            retained,
            point_cache,
            factor,
            vertices,
            ..
        } = worker.into_inner();
        drop(point_cache);
        drop(factor);
        let output_receipt = retained.finish(result.is_ok(), context, &mut progress)?;
        Ok(GeometryImage {
            literal: Arc::new(GeometryLiteral::new(target, result?)),
            chain: chain_id,
            source: source_id,
            law: GeometryImage::output_law_id(),
            vertices,
            source_vertices: inventory.vertices,
            source_nesting_depth: inventory.nesting_depth,
            output_receipt: output_receipt.expect("successful complete image owns a receipt"),
        })
    }
}

struct ImageWorker<'input, 'context, 'progress, 'observer> {
    chain: &'input OperationChain,
    factor: Rat,
    epoch: Option<&'input Rat>,
    context: &'context mut MetricContext,
    progress: &'progress mut WorkProgress<'observer>,
    vertices: u64,
    retained: MaterializationStorage,
    point_cache: purrdf_core::FastMap<u64, Vec<(Coord, Coord)>>,
}
impl ImageWorker<'_, '_, '_, '_> {
    fn progress_poll(&mut self) -> Result<(), GeoError> {
        self.progress.context_poll(self.context)
    }
    fn admit_vertex(&mut self, coordinate: &Coord) -> Result<(), GeoError> {
        let next = self
            .vertices
            .checked_add(1)
            .ok_or(GeoError::ArithmeticOverflow("image vertices"))?;
        if next > self.context.policy().limits().max_output_elements {
            return Err(GeoError::OutputExhausted {
                limit: self.context.policy().limits().max_output_elements,
            });
        }
        let mut bytes = (size_of::<Coord>() as u64)
            .saturating_mul(2)
            .saturating_add(64);
        for value in [
            Some(coordinate.x()),
            Some(coordinate.y()),
            coordinate.z(),
            coordinate.m(),
        ]
        .into_iter()
        .flatten()
        {
            let limbs = value
                .numerator()
                .bit_len()
                .checked_add(value.denominator().bit_len())
                .and_then(|bits| bits.div_ceil(8).checked_mul(4))
                .ok_or(GeoError::ArithmeticOverflow("image coordinate storage"))?;
            let allocated = u64::try_from(value.allocated_bytes())
                .ok()
                .and_then(|bytes| bytes.checked_mul(2))
                .ok_or(GeoError::ArithmeticOverflow(
                    "retained image coordinate storage",
                ))?;
            bytes = bytes
                .checked_add(limbs.max(allocated))
                .ok_or(GeoError::ArithmeticOverflow(
                    "complete image coordinate storage",
                ))?;
        }
        self.retained.admit_output(bytes, self.context)?;
        self.vertices = next;
        self.context.charge_work(1)?;
        self.progress_poll()
    }
    fn point(&mut self, coordinate: &Coord) -> Result<Coord, GeoError> {
        let operands = [
            Some(coordinate.x()),
            Some(coordinate.y()),
            coordinate.z(),
            coordinate.m(),
        ]
        .into_iter()
        .flatten()
        .collect::<purrdf_core::SmallVec<[&Rat; 4]>>();
        // Hash the original canonical limbs once. Exact collision comparisons
        // are admitted separately for the actual bucket, so unrelated cached
        // points do not charge a quadratic all-table comparison bound.
        let cost = crate::numerical::rational_cost(
            purrdf_xsd::integer::ExactOperation::Linear,
            &operands,
            operands.len() as u64,
        )
        .ok_or(GeoError::ArithmeticOverflow("original image cache lookup"))?;
        let hash = self.progress.exact(self.context, cost, || {
            use core::hash::{Hash, Hasher};
            let mut hasher = purrdf_hash::fixed::FixedHasher::default();
            coordinate.hash(&mut hasher);
            Ok(hasher.finish())
        })?;
        let mut cached_output = None;
        if let Some(bucket) = self.point_cache.get(&hash) {
            for (key, cached) in bucket {
                let keys = operands
                    .iter()
                    .copied()
                    .chain(
                        [Some(key.x()), Some(key.y()), key.z(), key.m()]
                            .into_iter()
                            .flatten(),
                    )
                    .collect::<purrdf_core::SmallVec<[&Rat; 8]>>();
                let same = crate::numerical::ExactAdmission::new(self.context, self.progress)
                    .rational(
                        purrdf_xsd::integer::ExactOperation::Linear,
                        &keys,
                        keys.len() as u64,
                        || Ok(key == coordinate),
                    )?;
                if !same {
                    continue;
                }
                let values = [Some(cached.x()), Some(cached.y()), cached.z(), cached.m()]
                    .into_iter()
                    .flatten()
                    .collect::<purrdf_core::SmallVec<[&Rat; 4]>>();
                cached_output = Some(
                    crate::numerical::ExactAdmission::new(self.context, self.progress).rational(
                        purrdf_xsd::integer::ExactOperation::Linear,
                        &values,
                        values.len() as u64,
                        || Ok(cached.clone()),
                    )?,
                );
                break;
            }
        }
        if let Some(output) = cached_output {
            self.admit_vertex(&output)?;
            return Ok(output);
        }
        let measure = if let Some(measure) = coordinate.m() {
            let cost = crate::numerical::rational_cost(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[measure],
                1,
            )
            .ok_or(GeoError::ArithmeticOverflow("image measure copy"))?;
            Some(
                self.progress
                    .exact(self.context, cost, || Ok(measure.clone()))?,
            )
        } else {
            None
        };
        let point = self
            .chain
            .apply_source_coordinate_admitted(coordinate, self.epoch, self.context, self.progress)?
            .into_point();
        let output = Coord::new(point.x, point.y, point.z, measure);
        self.admit_vertex(&output)?;
        let values = operands
            .into_iter()
            .chain(
                [Some(output.x()), Some(output.y()), output.z(), output.m()]
                    .into_iter()
                    .flatten(),
            )
            .collect::<purrdf_core::SmallVec<[&Rat; 8]>>();
        let cost = crate::numerical::rational_cost(
            purrdf_xsd::integer::ExactOperation::Linear,
            &values,
            values.len() as u64,
        )
        .ok_or(GeoError::ArithmeticOverflow(
            "original image cache retention",
        ))?;
        let bytes = cost.workspace_bytes.saturating_add(
            (size_of::<(Coord, Coord)>() as u64)
                .saturating_mul(4)
                .saturating_add(64),
        );
        self.retained.admit_transient(bytes, self.context)?;
        self.progress.context_poll(self.context)?;
        self.point_cache
            .try_reserve(1)
            .map_err(|_| GeoError::MemoryExhausted {
                limit: self.context.policy().limits().max_workspace_bytes,
            })?;
        let limit = self.context.policy().limits().max_workspace_bytes;
        self.progress.exact(self.context, cost, || {
            let bucket = self.point_cache.entry(hash).or_default();
            bucket
                .try_reserve(1)
                .map_err(|_| GeoError::MemoryExhausted { limit })?;
            bucket.push((coordinate.clone(), output.clone()));
            Ok(())
        })?;
        drop(values);
        Ok(output)
    }
    fn leaf(&mut self, node: &Geometry) -> Result<Geometry, GeoError> {
        self.context.charge_work(1)?;
        self.progress_poll()?;
        let body = match node.body() {
            GeometryBody::Point(point) => {
                GeometryBody::Point(point.as_ref().map(|point| self.point(point)).transpose()?)
            }
            GeometryBody::MultiPoint(points) => GeometryBody::MultiPoint(
                points
                    .iter()
                    .map(|point| point.as_ref().map(|point| self.point(point)).transpose())
                    .collect::<Result<_, _>>()?,
            ),
            GeometryBody::LineString(line) => GeometryBody::LineString(self.line(line)?),
            GeometryBody::MultiLineString(lines) => GeometryBody::MultiLineString(
                lines
                    .iter()
                    .map(|line| self.line(line))
                    .collect::<Result<_, _>>()?,
            ),
            GeometryBody::Polygon(rings) => {
                if !self.chain.registered_injective_area() {
                    return self.folded_area(node.dim(), rings);
                }
                GeometryBody::Polygon(
                    rings
                        .iter()
                        .map(|ring| self.line(ring))
                        .collect::<Result<_, _>>()?,
                )
            }
            GeometryBody::MultiPolygon(polygons) => {
                if !self.chain.registered_injective_area() {
                    let mut members = Vec::new();
                    for rings in polygons {
                        members.push(self.folded_area(node.dim(), rings)?);
                    }
                    return Geometry::new(node.dim(), GeometryBody::GeometryCollection(members));
                }
                GeometryBody::MultiPolygon(
                    polygons
                        .iter()
                        .map(|rings| {
                            rings
                                .iter()
                                .map(|ring| self.line(ring))
                                .collect::<Result<_, _>>()
                        })
                        .collect::<Result<_, _>>()?,
                )
            }
            GeometryBody::GeometryCollection(_) => unreachable!("shared bottom-up collection walk"),
        };
        Geometry::new(node.dim(), body)
    }
    fn line(&mut self, source: &[Coord]) -> Result<Vec<Coord>, GeoError> {
        let Some(first) = source.first() else {
            return Ok(Vec::new());
        };
        let mut output = vec![self.point(first)?];
        let stack_bytes = (u64::from(self.context.policy().limits().max_subdivision_levels) + 2)
            .checked_mul(4096)
            .ok_or(GeoError::ArithmeticOverflow("image panel stack"))?;
        self.context.admit_workspace(stack_bytes)?;
        let result = (|| {
            for edge in source.windows(2) {
                let mut pending = purrdf_lex::walk::WorkList::<(Rat, Rat, u32), 8>::with((
                    Rat::zero(),
                    Rat::one(),
                    0,
                ));
                while let Some((lower, upper, depth)) = pending.pop() {
                    self.context.charge_work(1)?;
                    self.progress_poll()?;
                    let mut admission =
                        crate::numerical::ExactAdmission::new(self.context, self.progress);
                    let a = crate::SourceLinearEdge::interpolate_coord_admitted(
                        &edge[0],
                        &edge[1],
                        &lower,
                        &mut admission,
                    )?;
                    let b = crate::SourceLinearEdge::interpolate_coord_admitted(
                        &edge[0],
                        &edge[1],
                        &upper,
                        &mut admission,
                    )?;
                    let accepted = match self.panel_bound(&a, &b) {
                        Ok(bound) => {
                            bound
                                <= Rat::parse_decimal("0.099998")
                                    .expect("frozen complete-image target")
                        }
                        Err(GeoError::PrecisionExhausted { .. }) => false,
                        Err(error) => return Err(error),
                    };
                    if accepted {
                        output.push(self.point(&b)?);
                    } else {
                        if depth >= self.context.policy().limits().max_subdivision_levels {
                            return Err(GeoError::PrecisionExhausted {
                                bits: self.context.policy().limits().max_precision_bits,
                            });
                        }
                        use purrdf_xsd::integer::ExactOperation::{
                            Linear, RationalAdd, RationalDivide,
                        };
                        let mut admission =
                            crate::numerical::ExactAdmission::new(self.context, self.progress);
                        let sum = admission.rational(RationalAdd, &[&lower, &upper], 1, || {
                            Ok(lower.add(&upper))
                        })?;
                        let two = Rat::from_i64(2);
                        let middle =
                            admission.rational(RationalDivide, &[&sum, &two], 1, || {
                                Ok(sum.div(&two).expect("positive divisor"))
                            })?;
                        let copy =
                            admission.rational(Linear, &[&middle], 1, || Ok(middle.clone()))?;
                        pending.push((copy, upper, depth + 1));
                        pending.push((lower, middle, depth + 1));
                    }
                }
            }
            Ok(output)
        })();
        self.context.release_workspace(stack_bytes)?;
        result
    }
    fn panel_bound(&mut self, a: &Coord, b: &Coord) -> Result<Rat, GeoError> {
        let policy = self.context.policy();
        let bytes = (96usize.div_ceil(8) * 8 + 128) * 192;
        let chain = self.chain;
        let factor = &self.factor;
        let epoch = self.epoch;
        let solver = OperationSolverLimits {
            iterations: policy.limits().max_iterations,
            subdivisions: policy.limits().max_subdivision_levels,
            quantize_inverse: false,
        };
        crate::numerical::with_math(self.context, self.progress, 96, bytes, |math, progress| {
            Self::panel_math(chain, factor, epoch, (a, b), solver, math, progress)
                .map_err(|error| geo_math_error(&error, policy))
        })
    }
    fn panel_math(
        chain: &OperationChain,
        factor: &Rat,
        epoch: Option<&Rat>,
        (a, b): (&Coord, &Coord),
        solver: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Rat, MathError> {
        let panel = evaluate_panel(
            chain,
            epoch,
            (a, b),
            None,
            (solver, PanelKind::Directional),
            math,
            progress,
        )?;
        let [dx, dy] = [(a.x(), b.x()), (a.y(), b.y())].map(|(a, b)| {
            let delta = crate::numerical::math_exact_rational(
                purrdf_xsd::integer::ExactOperation::RationalAdd,
                &[a, b],
                math,
                progress,
                || b.sub(a),
            )?;
            fixed_from_rat(&delta, math)
        });
        let mut delta = [dx?, dy?];
        if chain
            .operations()
            .first()
            .expect("compiled chain")
            .source()
            .swapped_axes
        {
            delta.swap(0, 1);
        }
        let mut bound = FixedInterval::from_i64(0, math)?;
        let half = FixedInterval::from_i64(2, math)?;
        if let Some(jacobian) = panel.jacobian {
            for row in &jacobian {
                let mut range = FixedInterval::from_i64(0, math)?;
                for (entry, delta) in row.iter().zip(&delta) {
                    range = range.add(&entry.width(math)?.mul(&delta.abs(math)?, math)?, math)?;
                }
                bound = bound.add(&range.div(&half, math)?, math)?;
            }
        } else if let Some(derivative) = panel.directional {
            for coordinate in derivative {
                bound = bound.add(&coordinate.width(math)?.div(&half, math)?, math)?;
            }
        } else {
            for coordinate in panel.coordinates.iter().take(2) {
                bound = bound.add(&coordinate.width(math)?, math)?;
            }
        }
        Ok(exact_bounds(&bound.mul(&fixed_from_rat(factor, math)?, math)?).1)
    }
}

/// Complete smooth physical normal and an optional original-parameter derivative.
pub(crate) type OperationNormalEnclosure = ([FixedInterval; 3], Option<[FixedInterval; 3]>);

struct PanelEnclosure {
    coordinates: OperationCoordinates,
    jacobian: Option<RootJacobian2>,
    directional: Option<[FixedInterval; 2]>,
    normal: Option<OperationNormalEnclosure>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PanelKind {
    Image,
    Jacobian,
    Directional,
    Normal,
}

fn source_delta(
    start: &Coord,
    end: &Coord,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<purrdf_core::SmallVec<[Rat; 3]>, MathError> {
    [
        Some((start.x(), end.x())),
        Some((start.y(), end.y())),
        start.z().zip(end.z()),
    ]
    .into_iter()
    .flatten()
    .map(|(start, end)| {
        crate::numerical::math_exact_rational(
            purrdf_xsd::integer::ExactOperation::RationalAdd,
            &[start, end],
            math,
            progress,
            || end.sub(start),
        )
    })
    .collect()
}

fn evaluate_panel(
    chain: &OperationChain,
    epoch: Option<&Rat>,
    (a, b): (&Coord, &Coord),
    original_delta: Option<&[Rat]>,
    (solver, kind): (OperationSolverLimits, PanelKind),
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<PanelEnclosure, MathError> {
    evaluate_panel_through(
        chain,
        epoch,
        (a, b),
        original_delta,
        (solver, kind, chain.operations().len()),
        math,
        progress,
    )
}

fn evaluate_panel_through(
    chain: &OperationChain,
    epoch: Option<&Rat>,
    (a, b): (&Coord, &Coord),
    original_delta: Option<&[Rat]>,
    (solver, kind, count): (OperationSolverLimits, PanelKind, usize),
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<PanelEnclosure, MathError> {
    if count == 0 || count > chain.operations().len() {
        return Err(MathError::Domain(
            "operation prefix length outside compiled chain",
        ));
    }
    let first = chain.operations().first().expect("compiled chain");
    let point = chain.prepare_source_coordinate_math(a, epoch, math, progress)?;
    let mut coordinates = OperationCoordinates::default();
    for (left, right) in [(a.x(), b.x()), (a.y(), b.y())]
        .into_iter()
        .chain(a.z().zip(b.z()))
    {
        let order = crate::numerical::math_exact_rational(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            &[left, right],
            math,
            progress,
            || left.cmp(right),
        )?;
        let (lower, upper) = if order.is_gt() {
            (right, left)
        } else {
            (left, right)
        };
        coordinates.push(fixed_from_bounds(lower, upper, math)?);
    }
    if first.source().swapped_axes {
        coordinates.swap(0, 1);
    }
    let mut jacobian = identity_matrix(math)?;
    let constant_height = if let (Some(a), Some(b)) = (a.z(), b.z()) {
        crate::numerical::math_exact_rational(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[a, b],
            math,
            progress,
            || a == b,
        )?
    } else {
        true
    };
    let mut has_jacobian = kind != PanelKind::Image && constant_height;
    let mut tangent = if matches!(kind, PanelKind::Directional | PanelKind::Normal) {
        let mut delta = if let Some(delta) = original_delta {
            let operands = delta.iter().collect::<purrdf_core::SmallVec<[&Rat; 3]>>();
            crate::numerical::math_exact_rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &operands,
                math,
                progress,
                || {
                    delta
                        .iter()
                        .cloned()
                        .collect::<purrdf_core::SmallVec<[Rat; 3]>>()
                },
            )?
        } else {
            source_delta(a, b, math, progress)?
        };
        if first.source().swapped_axes {
            delta.swap(0, 1);
        }
        Some(
            delta
                .iter()
                .map(|value| fixed_from_rat(value, math))
                .collect::<Result<OperationCoordinates, _>>()?,
        )
    } else {
        None
    };
    if count == chain.operations().len()
        && let Some(parameters) = chain.exact_tm_roundtrip()
    {
        // For the original holomorphic TM map F(q+iλ), D=F' satisfies
        // (log D)'=-sinφ. The original continuation proof gives |Imφ|<3/4;
        // hence |arg D| <= cosh(3/4)|λ| < (4/3)(π/60)=π/45.
        // Re D>0 throughout the convex native nonpolar strip. Integrating D
        // on the straight segment between any two points proves F injective.
        // Its explicitly matching all-root inverse is therefore the identity
        // on this complete source panel. No rounded projected carrier or
        // inferred realization/zone participates in this proof.
        parameters.validate_interval(&coordinates)?;
        let south = FixedInterval::from_i64(-90, math)?;
        let north = FixedInterval::from_i64(90, math)?;
        if coordinates[1].lower() > south.upper() && coordinates[1].upper() < north.lower() {
            return Ok(PanelEnclosure {
                coordinates,
                jacobian: has_jacobian.then_some(jacobian),
                directional: tangent.map(|rate| [rate[0].clone(), rate[1].clone()]),
                normal: None,
            });
        }
    }
    for (index, operation) in chain.operations()[..count].iter().enumerate() {
        if kind == PanelKind::Normal
            && index + 1 == count
            && let OperationModel::GeocentricToGeographic { ellipsoid } = operation.model()
        {
            let normal = directional::geocentric_normal(
                &coordinates,
                tangent.as_deref(),
                ellipsoid,
                solver,
                math,
                progress,
            )?;
            progress.math_poll(math)?;
            // The normal is the complete target image. The original XYZ box
            // remains available internally; no longitude chart is evaluated.
            return Ok(PanelEnclosure {
                coordinates,
                jacobian: None,
                directional: None,
                normal: Some(normal),
            });
        }
        if kind == PanelKind::Image {
            coordinates = operation.model().evaluate_fixed(
                &point,
                &coordinates,
                false,
                solver,
                math,
                progress,
            )?;
            progress.math_poll(math)?;
            continue;
        }
        let three = directional::evaluate(
            operation.model(),
            &point,
            (&coordinates, tangent.as_deref()),
            solver,
            math,
            progress,
        )?;
        let (image, differential) = if let Some(image) = three {
            tangent = image.derivative;
            (image.coordinates, None)
        } else {
            let (image, jacobian) = differential(
                operation.model(),
                &point,
                &coordinates,
                solver,
                math,
                progress,
            )?;
            tangent = match (&tangent, &jacobian) {
                (Some(rate), Some(jac)) => {
                    let mut next = purrdf_core::smallvec![
                        jac[0][0]
                            .mul(&rate[0], math)?
                            .add(&jac[0][1].mul(&rate[1], math)?, math)?,
                        jac[1][0]
                            .mul(&rate[0], math)?
                            .add(&jac[1][1].mul(&rate[1], math)?, math)?,
                    ];
                    if let Some(height) = rate.get(2) {
                        next.push(height.clone());
                    }
                    Some(next)
                }
                _ => None,
            };
            (image, jacobian)
        };
        if let Some(next) = differential {
            jacobian = purrdf_xsd::math::compose_jacobians2(&next, &jacobian, math)?;
        } else {
            has_jacobian = false;
        }
        coordinates = image;
        progress.math_poll(math)?;
    }
    Ok(PanelEnclosure {
        coordinates,
        jacobian: has_jacobian.then_some(jacobian),
        directional: tangent.map(|rate| [rate[0].clone(), rate[1].clone()]),
        normal: None,
    })
}

fn identity_matrix(math: &mut CoordinateMath) -> Result<RootJacobian2, MathError> {
    Ok([
        [
            FixedInterval::from_i64(1, math)?,
            FixedInterval::from_i64(0, math)?,
        ],
        [
            FixedInterval::from_i64(0, math)?,
            FixedInterval::from_i64(1, math)?,
        ],
    ])
}
fn differential(
    model: &OperationModel,
    point: &OperationPoint,
    coordinates: &[FixedInterval],
    limits: OperationSolverLimits,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<(OperationCoordinates, Option<RootJacobian2>), MathError> {
    match model {
        OperationModel::GcjRationalHarmonicV1(_)
        | OperationModel::GcjRationalHarmonicInverseV1(_)
        | OperationModel::Bd09LlV1
        | OperationModel::Bd09LlInverseV1
        | OperationModel::Polynomial2d(_)
        | OperationModel::Polynomial2dInverse(_)
        | OperationModel::BilinearGrid(_)
        | OperationModel::BilinearGridInverse(_) => {
            model.differential(point, coordinates, limits, math, progress)
        }
        OperationModel::TransverseMercator(model) => model
            .evaluate_differential(point, coordinates, false, math, progress)
            .map(|(image, jac)| (image, Some(jac))),
        OperationModel::TransverseMercatorInverse(parameters) => {
            let image = model.evaluate_fixed(point, coordinates, false, limits, math, progress)?;
            let jacobian = parameters
                .evaluate_differential(point, &image, false, math, progress)?
                .1;
            Ok((
                image,
                Some(purrdf_xsd::math::invert_jacobian2(&jacobian, math)?),
            ))
        }
        OperationModel::EllipsoidalMercator {
            semimajor,
            eccentricity_squared,
        } => {
            let image = model.evaluate_fixed(point, coordinates, false, limits, math, progress)?;
            let jac = mercator_jacobian(
                semimajor,
                eccentricity_squared,
                &coordinates[1],
                false,
                math,
            )?;
            Ok((image, Some(jac)))
        }
        OperationModel::BaiduMercatorAnalyticV1 => {
            let image = model.evaluate_fixed(point, coordinates, false, limits, math, progress)?;
            let (semimajor, eccentricity_squared) = super::baidu_mercator_parameters();
            let jac = mercator_jacobian(
                &semimajor,
                &eccentricity_squared,
                &coordinates[1],
                false,
                math,
            )?;
            Ok((image, Some(jac)))
        }
        OperationModel::WebMercator { radius } => {
            let image = model.evaluate_fixed(point, coordinates, false, limits, math, progress)?;
            let jac = mercator_jacobian(radius, &Rat::zero(), &coordinates[1], false, math)?;
            Ok((image, Some(jac)))
        }
        OperationModel::MercatorToGeographic {
            radius,
            eccentricity_squared,
            ..
        } => {
            let image = model.evaluate_fixed(point, coordinates, false, limits, math, progress)?;
            let jac = mercator_jacobian(radius, eccentricity_squared, &image[1], true, math)?;
            Ok((image, Some(jac)))
        }
        OperationModel::Similarity2d(model) => {
            let zero = FixedInterval::from_i64(0, math)?;
            let one = FixedInterval::from_i64(1, math)?;
            let origin = model.evaluate(&[zero.clone(), zero.clone()], math)?;
            let x = model.evaluate(&[one.clone(), zero.clone()], math)?;
            let y = model.evaluate(&[zero, one], math)?;
            let jac = [
                [x[0].sub(&origin[0], math)?, y[0].sub(&origin[0], math)?],
                [x[1].sub(&origin[1], math)?, y[1].sub(&origin[1], math)?],
            ];
            Ok((model.evaluate(coordinates, math)?, Some(jac)))
        }
        _ => Ok((
            model.evaluate_fixed(point, coordinates, false, limits, math, progress)?,
            None,
        )),
    }
}

// Differentiate the declared ellipsoidal isometric latitude exactly:
// dy/dphi = a(1-e²)/(cos(phi)(1-e²sin²(phi))). These interval derivatives
// bound the complete panel and are independent of point-coordinate rounding.
fn mercator_jacobian(
    a: &Rat,
    e2: &Rat,
    latitude: &FixedInterval,
    inverse: bool,
    math: &mut CoordinateMath,
) -> Result<RootJacobian2, MathError> {
    let one = FixedInterval::from_i64(1, math)?;
    let zero = FixedInterval::from_i64(0, math)?;
    let factor = FixedInterval::pi(math)?.div(&FixedInterval::from_i64(180, math)?, math)?;
    let phi = super::radians(latitude, math)?;
    let (sine, cosine) = phi.sin_cos_range(math)?;
    let e2 = fixed_from_rat(e2, math)?;
    let a = fixed_from_rat(a, math)?;
    let dx = a.mul(&factor, math)?;
    let dy = a
        .mul(&one.sub(&e2, math)?, math)?
        .div(
            &cosine.mul(&one.sub(&e2.mul(&sine.square(math)?, math)?, math)?, math)?,
            math,
        )?
        .mul(&factor, math)?;
    let (dx, dy) = if inverse {
        (one.div(&dx, math)?, one.div(&dy, math)?)
    } else {
        (dx, dy)
    };
    Ok([[dx, zero.clone()], [zero, dy]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::{
        Applicability, CoordinateOperation, OperationReference, Polynomial2d, PolynomialTerm,
        RotationConvention, Similarity2d,
    };
    use purrdf_hash::hex::Digest32;
    fn rat(text: &str) -> Rat {
        Rat::parse_decimal(text).unwrap()
    }
    fn chain(model: OperationModel) -> OperationChain {
        let reference = |id| OperationReference {
            realization: Digest32::new([id; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: false,
        };
        OperationChain::compile(vec![
            CoordinateOperation::compile(reference(1), reference(2), model).unwrap(),
        ])
        .unwrap()
    }
    fn crs() -> Crs {
        Crs::new("https://example.org/cartesian").unwrap()
    }
    #[test]
    fn materialized_image_storage_survives_a_refused_export() {
        let chain = chain(OperationModel::Similarity2d(Similarity2d {
            translation: [Rat::zero(), Rat::zero()],
            scale: Rat::one(),
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        }));
        let source = crate::wkt::parse(
            "GEOMETRYCOLLECTION(GEOMETRYCOLLECTION(MULTIPOINT(EMPTY,EMPTY)),LINESTRING(0 0,1 1))",
            &crs(),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let image = chain
            .materialize_geometry(
                source.geometry(),
                crs(),
                &ImageMetric::CartesianMetres,
                None,
                &mut context,
            )
            .unwrap();
        let bytes = image.retained_workspace_bytes();
        assert!(bytes >= (size_of::<GeometryImage>() + 2 * size_of::<Geometry>()) as u64);
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            context.policy().limits().max_workspace_bytes
        );
        let registered = context.retained_workspace_bytes();
        context.begin(1).unwrap();
        assert_eq!(context.retained_workspace_bytes(), registered);
        assert!(matches!(
            context.release_materialized_output(image.output_receipt()),
            Err(GeoError::Config(_))
        ));
        let remaining = context.remaining_workspace();
        context.admit_workspace(remaining).unwrap();
        assert!(matches!(
            crate::wkt::write_in_context(image.literal(), 6, &mut context),
            Err(GeoError::MemoryExhausted { .. })
        ));
        assert_eq!(context.remaining_workspace(), 0);
        context.release_workspace(remaining).unwrap();
        let literal = image.literal().clone();
        let law = image.law_id();
        let certificate = image.certificate_bytes();
        let shared = image.clone();
        assert!(core::ptr::eq(image.literal(), shared.literal()));
        let receipt = image.output_receipt();
        drop(image);
        assert!(matches!(
            context.release_materialized_output(receipt.clone()),
            Err(GeoError::Config(_))
        ));
        drop(shared);
        context.release_materialized_output(receipt).unwrap();
        assert_eq!(context.retained_workspace_bytes() + bytes, registered);
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            context.policy().limits().max_workspace_bytes
        );
        let second = chain
            .materialize_geometry(
                source.geometry(),
                crs(),
                &ImageMetric::CartesianMetres,
                None,
                &mut context,
            )
            .unwrap();
        assert_eq!(second.literal(), &literal);
        assert_eq!(second.law_id(), law);
        assert_eq!(second.certificate_bytes(), certificate);
    }

    #[test]
    fn continuous_gcj_cusp_uses_complete_image_cells_without_inventing_a_derivative() {
        let target_reference = GeographicReference::new(
            crate::PreparedEllipsoid::wgs84(),
            Digest32::new([17; 32]),
            crate::AxisOrder::LonLat,
        );
        let reference = |realization| OperationReference {
            realization,
            unit: CoordinateUnit::Degrees,
            swapped_axes: false,
        };
        let chain = OperationChain::compile(vec![
            CoordinateOperation::compile(
                reference(GeographicReference::wgs84().id().digest()),
                reference(target_reference.id().digest()),
                OperationModel::GcjRationalHarmonicV1(
                    Applicability::new(rat("104"), rat("106"), rat("34"), rat("36")).unwrap(),
                ),
            )
            .unwrap(),
        ])
        .unwrap();
        let ring = [
            ("104.999998", "34.9999995"),
            ("105.000002", "34.9999995"),
            ("105.000002", "35.0000005"),
            ("104.999998", "35.0000005"),
            ("104.999998", "34.9999995"),
        ]
        .map(|(x, y)| Coord::xy(rat(x), rat(y)))
        .to_vec();
        let source = Geometry::new(crate::CoordDim::Xy, GeometryBody::Polygon(vec![ring])).unwrap();
        let target_crs = Crs::new("https://example.org/declared-gcj").unwrap();
        let metric = ImageMetric::Geographic(Box::new(target_reference.clone()));
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items = 4_000_000;
        let mut context = MetricContext::new(
            target_reference.clone(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let image = chain
            .materialize_geometry(&source, target_crs.clone(), &metric, None, &mut context)
            .expect("continuous square-root cusp has a complete image");
        for x in ["104.999999", "105", "105.000001"] {
            for y in ["34.9999999", "35", "35.0000001"] {
                let transformed = chain
                    .apply(
                        &OperationPoint {
                            x: rat(x),
                            y: rat(y),
                            z: None,
                            epoch: None,
                        },
                        &mut context,
                    )
                    .unwrap();
                let point = transformed.point();
                assert_ne!(
                    crate::topology::locate(
                        &Coord::xy(point.x.clone(), point.y.clone()),
                        image.literal().geometry(),
                    ),
                    crate::Set::Exterior,
                );
            }
        }
        assert_eq!(image.error_bound().exact(), &rat("0.1"));
        let factor = metric
            .factor(&mut context, &mut WorkProgress::new(None))
            .unwrap()
            .mul(&Rat::from_i64(2));
        for x in ["104.999998", "105", "105.000002"] {
            for y in ["34.9999995", "35.0000005"] {
                let point = chain
                    .apply(
                        &OperationPoint {
                            x: rat(x),
                            y: rat(y),
                            z: None,
                            epoch: None,
                        },
                        &mut context,
                    )
                    .unwrap()
                    .into_point();
                let point = Geometry::new(
                    crate::CoordDim::Xy,
                    GeometryBody::Point(Some(Coord::xy(point.x, point.y))),
                )
                .unwrap();
                // The independent exact planar metric truncates by less than
                // 1e-18 degree. L1 <= 2 L2 converts its outward upper bound to
                // ground distance; scalar output quantization adds 1um.
                let upper = crate::measure::distance(&point, image.literal().geometry())
                    .unwrap()
                    .add(&rat("0.000000000000000001"))
                    .mul(&factor)
                    .add(&rat("0.000001"));
                assert!(upper < rat("0.1"));
            }
        }
        limits.max_work_items *= 2;
        let mut raised = MetricContext::new(
            target_reference,
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert_eq!(
            image,
            chain
                .materialize_geometry(&source, target_crs, &metric, None, &mut raised)
                .unwrap(),
        );
    }
    #[test]
    fn folded_polynomial_materializes_the_complete_original_region_image() {
        let fold = Polynomial2d::new(
            2,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            vec![
                PolynomialTerm {
                    x_power: 2,
                    y_power: 0,
                    x_coefficient: Rat::one(),
                    y_coefficient: Rat::zero(),
                },
                PolynomialTerm {
                    x_power: 0,
                    y_power: 1,
                    x_coefficient: Rat::zero(),
                    y_coefficient: Rat::one(),
                },
            ],
        )
        .unwrap();
        let chain = chain(OperationModel::Polynomial2d(fold));
        let ring = [
            ("-0.2", "0"),
            ("0.2", "0"),
            ("0.2", "0.2"),
            ("-0.2", "0.2"),
            ("-0.2", "0"),
        ]
        .map(|(x, y)| Coord::xy(rat(x), rat(y)))
        .to_vec();
        let source = Geometry::new(crate::CoordDim::Xy, GeometryBody::Polygon(vec![ring])).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let image = chain
            .materialize_geometry(
                &source,
                crs(),
                &ImageMetric::CartesianMetres,
                None,
                &mut context,
            )
            .expect("complete image across the critical curve x=0");
        assert!(matches!(
            image.literal().geometry().body(),
            GeometryBody::GeometryCollection(_)
        ));
        // The exact image is [0,.04]x[0,.2]. Both folded preimages are covered,
        // and every output position lies within the stated physical band.
        for x in ["0", "0.0001", "0.01", "0.04"] {
            for y in ["0", "0.025", "0.1", "0.175", "0.2"] {
                assert_ne!(
                    crate::topology::locate(&Coord::xy(rat(x), rat(y)), image.literal().geometry()),
                    crate::Set::Exterior,
                );
            }
        }
        for point in image.literal().geometry().coords() {
            assert!(point.x() >= &rat("-0.1") && point.x() <= &rat("0.14"));
            assert!(point.y() >= &rat("-0.1") && point.y() <= &rat("0.3"));
        }
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items *= 2;
        let mut raised = MetricContext::new(
            GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert_eq!(
            image,
            chain
                .materialize_geometry(
                    &source,
                    crs(),
                    &ImageMetric::CartesianMetres,
                    None,
                    &mut raised
                )
                .unwrap(),
        );
    }
    #[test]
    fn complete_affine_curve_keeps_measures_and_admission_independent_bytes() {
        let chain = chain(OperationModel::Similarity2d(Similarity2d {
            translation: [rat("1"), rat("2")],
            scale: rat("2"),
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        }));
        let source = Geometry::new(
            crate::CoordDim::Xyzm,
            GeometryBody::LineString(vec![
                Coord::new(rat("0"), rat("0"), Some(rat("3")), Some(rat("7"))),
                Coord::new(rat("1000"), rat("2000"), Some(rat("3")), Some(rat("9"))),
            ]),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let image = chain
            .materialize_geometry(
                &source,
                crs(),
                &ImageMetric::CartesianMetres,
                None,
                &mut context,
            )
            .unwrap();
        assert_eq!(image.vertices(), 2);
        let GeometryBody::LineString(line) = image.literal().geometry().body() else {
            panic!("line image")
        };
        assert_eq!(
            line[0],
            Coord::new(rat("1"), rat("2"), Some(rat("3")), Some(rat("7")))
        );
        assert_eq!(
            line[1],
            Coord::new(rat("2001"), rat("4002"), Some(rat("3")), Some(rat("9")))
        );
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items *= 2;
        let mut other = MetricContext::new(
            GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert_eq!(
            image,
            chain
                .materialize_geometry(
                    &source,
                    crs(),
                    &ImageMetric::CartesianMetres,
                    None,
                    &mut other
                )
                .unwrap()
        );
    }
    #[test]
    fn polynomial_complete_panel_certificate_resolves_curvature_without_samples() {
        let polynomial = Polynomial2d::new(
            2,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            vec![
                PolynomialTerm {
                    x_power: 1,
                    y_power: 0,
                    x_coefficient: Rat::one(),
                    y_coefficient: Rat::zero(),
                },
                PolynomialTerm {
                    x_power: 2,
                    y_power: 0,
                    x_coefficient: Rat::zero(),
                    y_coefficient: Rat::one(),
                },
            ],
        )
        .unwrap();
        let chain = chain(OperationModel::Polynomial2d(polynomial));
        let source = Geometry::new(
            crate::CoordDim::Xy,
            GeometryBody::LineString(vec![
                Coord::xy(rat("-1"), Rat::zero()),
                Coord::xy(Rat::one(), Rat::zero()),
            ]),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let image = chain
            .materialize_geometry(
                &source,
                crs(),
                &ImageMetric::CartesianMetres,
                None,
                &mut context,
            )
            .unwrap();
        let GeometryBody::LineString(line) = image.literal().geometry().body() else {
            panic!("line image")
        };
        assert!(line.len() > 2);
        for edge in line.windows(2) {
            let width = edge[1].x().sub(edge[0].x());
            // Independently analytic parabola deviation is width²/4, with the
            // vertical nearest point proving the same Euclidean upper bound.
            assert!(width.mul(&width).div(&rat("4")).unwrap() <= *image.error_bound().exact());
        }
        assert!(line.iter().any(|p| p.x().is_zero() && p.y().is_zero()));
    }
    #[test]
    fn cancelled_materialization_never_publishes_a_partial_curve() {
        let chain = chain(OperationModel::Similarity2d(Similarity2d {
            translation: [Rat::zero(), Rat::zero()],
            scale: Rat::one(),
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        }));
        let source = Geometry::new(
            crate::CoordDim::Xy,
            GeometryBody::Point(Some(Coord::xy(Rat::zero(), Rat::zero()))),
        )
        .unwrap();
        struct Cancel;
        impl MetricWorkObserver for Cancel {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        assert_eq!(
            chain.materialize_geometry_metered(
                &source,
                crs(),
                &ImageMetric::CartesianMetres,
                None,
                &mut MetricContext::wgs84().unwrap(),
                &mut Cancel
            ),
            Err(GeoError::Cancelled)
        );
    }
}
