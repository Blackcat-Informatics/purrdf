// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One immutable profile/session over the pure engine, with per-call worker scratch.

use std::sync::{Arc, LazyLock, Mutex};

use purrdf_geo_kernel::{
    ExecutionPolicy, GeoError, GeoProfile, MetricContext, PreparationBudget, PreparedGeodesic,
    binding::GeoQueryIdentity,
    cells::{CellId, CubeHilbertQ62V1},
};
use purrdf_lex::json::{self, Object, Value};
use purrdf_xsd::math::PreparedBinary64;

use super::{GeoCallError, GeoRequest, encode, invocation::Invocation, output};

/// A prepared immutable geographic binding, shareable across independent workers.
///
/// Thread-bound numerical scratch is created for the invoking worker, never kept
/// across a host callback, suspension, `await`, or a release of Python's GIL.
#[derive(Clone, Debug)]
pub struct GeoSession {
    profile: Arc<GeoProfile>,
    identity: Result<GeoQueryIdentity, GeoError>,
    prepared: Arc<Mutex<SessionPreparations>>,
}

#[derive(Debug)]
struct SessionPreparation {
    geodesic: PreparedGeodesic,
    arithmetic: PreparedBinary64,
}

#[derive(Debug, Default)]
struct SessionPreparations {
    entries: Vec<SessionPreparation>,
    retained_bytes: u64,
}

static STANDARD_SESSION: LazyLock<GeoSession> =
    LazyLock::new(|| GeoSession::new(GeoProfile::standard()));

impl Default for GeoSession {
    fn default() -> Self {
        STANDARD_SESSION.clone()
    }
}

impl GeoSession {
    /// Carry an already validated immutable native profile.
    #[must_use]
    pub fn new(profile: GeoProfile) -> Self {
        let mut budget = PreparationBudget::new(ExecutionPolicy::geometry());
        let identity = compile_identity(&profile, &mut budget);
        Self {
            profile: Arc::new(profile),
            identity,
            prepared: Arc::new(Mutex::new(SessionPreparations::default())),
        }
    }

    /// Compile immutable binding/query IDs with an explicit configuration policy.
    /// That admission is independent of the profile's later invocation policy.
    ///
    /// # Errors
    /// Returns a typed configuration/work/storage refusal before publishing a session.
    pub fn try_new(
        profile: GeoProfile,
        compile_policy: ExecutionPolicy,
    ) -> Result<Self, GeoCallError> {
        Self::try_new_in_budget(profile, PreparationBudget::new(compile_policy))
    }

    /// Copy a borrowed immutable profile and compile its session under one
    /// cumulative configuration policy.
    ///
    /// # Errors
    /// Refuses source copies or identity compilation before their allocations
    /// exceed the same admitted work and storage budget.
    pub fn try_from_profile(
        profile: &GeoProfile,
        compile_policy: ExecutionPolicy,
    ) -> Result<Self, GeoCallError> {
        let mut budget = PreparationBudget::new(compile_policy);
        let profile = profile.clone_in_budget(&mut budget)?;
        Self::try_new_in_budget(profile, budget)
    }

    fn try_new_in_budget(
        profile: GeoProfile,
        mut budget: PreparationBudget,
    ) -> Result<Self, GeoCallError> {
        let identity = compile_identity(&profile, &mut budget)?;
        Ok(Self {
            profile: Arc::new(profile),
            identity: Ok(identity),
            prepared: Arc::new(Mutex::new(SessionPreparations::default())),
        })
    }

    pub(super) fn identity(&self) -> Result<GeoQueryIdentity, GeoError> {
        self.identity.clone()
    }

    /// Prepare the same strict profile a query's `geo` option takes.
    ///
    /// # Errors
    ///
    /// Refuses invalid JSON records, references and execution limits.
    pub fn from_profile_str(text: &str) -> Result<Self, GeoCallError> {
        Self::from_profile_str_with_policy(text, ExecutionPolicy::geometry())
    }

    /// Decode the same strict profile with one explicit configuration policy
    /// spanning input, exact model validation, registration and cached identities.
    /// # Errors
    /// Refuses malformed profiles and cumulative configuration work/storage exhaustion.
    pub fn from_profile_str_with_policy(
        text: &str,
        compile_policy: ExecutionPolicy,
    ) -> Result<Self, GeoCallError> {
        let (profile, budget) = super::profile::profile_with_budget(text, compile_policy)?;
        Self::try_new_in_budget(profile, budget)
    }

    /// Effective profile for query options and host provenance.
    #[must_use]
    pub fn profile(&self) -> &GeoProfile {
        &self.profile
    }

    /// Retain the admitted immutable profile without copying its graph.
    /// Each snapshot shares the same source, bindings and policy while another
    /// host or asynchronous operation changes its own selected profile.
    #[must_use]
    pub fn profile_snapshot(&self) -> Arc<GeoProfile> {
        self.profile.clone()
    }

    /// Execute a typed request with no ambient resources.
    ///
    /// # Errors
    ///
    /// Carries the engine's typed refusal unchanged; no partial batch succeeds.
    pub fn call(&self, request: &GeoRequest) -> Result<Value, GeoCallError> {
        let invocation = Invocation::new(self.profile.policy());
        self.call_with_preparation(
            request,
            PreparationBudget::new(self.profile.policy()),
            &invocation,
        )
    }

    fn call_with_preparation(
        &self,
        request: &GeoRequest,
        budget: PreparationBudget,
        invocation: &Invocation,
    ) -> Result<Value, GeoCallError> {
        let mut budget = invocation.track_budget(budget);
        let identity = self.identity()?;
        if let Some((work, bytes)) = request.collection_admission()? {
            budget.retain(work, bytes)?;
        }
        if let Some((work, bytes)) = request.reference_admission()? {
            budget.retain(work, bytes)?;
        }
        let result = match request {
            GeoRequest::Cover(request) => {
                super::cover::call(&self.profile, request, *budget, invocation)?
            }
            GeoRequest::Operation { name } => {
                let binding = self.profile.operation(name)?;
                let names = (binding.name().as_str().len()
                    + binding.source().as_str().len()
                    + binding.target().as_str().len()) as u64;
                output::admit(
                    *budget,
                    output::record(5, names + 64)?
                        .with_child(output::operation_chain(binding.chain())?)
                        .ok_or(GeoError::ArithmeticOverflow("operation record layout"))?,
                )?;
                Object::new()
                    .with("name", binding.name().as_str())
                    .with("source_crs", binding.source().as_str())
                    .with("target_crs", binding.target().as_str())
                    .with("id", binding.chain().id().to_string())
                    .with("chain", super::operation::chain_json(binding.chain())?)
                    .into()
            }
            GeoRequest::Transform { name, point, grid } => {
                let binding = self.profile.operation(name)?;
                let mut context = invocation.context(
                    budget.worker_context(purrdf_geo_kernel::binding::standard_reference())?,
                );
                let answer = binding
                    .chain()
                    .apply_with_grid(point, *grid, &mut context)?;
                output::admit_context(&mut context, output::transform(&answer)?)?;
                encode::transform(
                    &answer,
                    binding
                        .chain()
                        .operations()
                        .last()
                        .expect("compiled nonempty chain")
                        .target()
                        .unit,
                )?
            }
            GeoRequest::TransformGeometry {
                name,
                geometry,
                epoch,
            } => {
                let parsed = purrdf_geo_kernel::carrier::geometry_arg_in_policy(
                    purrdf_geo_kernel::standard_vocabulary(),
                    geometry,
                    budget.remaining()?,
                )?;
                budget.retain(
                    parsed.receipt().work_items(),
                    parsed.receipt().workspace_bytes(),
                )?;
                let mut context = invocation.context(
                    budget.worker_context(purrdf_geo_kernel::binding::standard_reference())?,
                );
                let image = self.profile.transform_literal_named(
                    name,
                    parsed.literal(),
                    epoch.as_ref(),
                    &mut context,
                )?;
                let lexical = purrdf_geo_kernel::wkt::write_in_context(
                    image.literal(),
                    self.profile
                        .operation_reference(image.literal().crs())?
                        .unit
                        .output_decimal_places(0),
                    &mut context,
                )?;
                let term = purrdf_geo_kernel::carrier::term_in_context(
                    lexical,
                    purrdf_geo_kernel::carrier::CarrierFormat::Wkt,
                    &mut context,
                )?;
                let (source_vertices, source_depth) = image.source_counts();
                output::admit_context(
                    &mut context,
                    output::record(9, 1024)?
                        .with_child(output::term(&term)?)
                        .ok_or(GeoError::ArithmeticOverflow("image output layout"))?,
                )?;
                let result = Object::new()
                    .with("geometry", encode::term(&term)?)
                    .with("operation", image.operation_id().to_string())
                    .with("source", image.source_id().to_string())
                    .with("law", image.law_id().digest().to_string())
                    .with(
                        "certificate",
                        purrdf_hash::hex::encode(&image.certificate_bytes()),
                    )
                    .with(
                        "error_bound_metres",
                        super::profile::exact_decimal(image.error_bound().exact())?,
                    )
                    .with("vertices", image.vertices())
                    .with("source_vertices", source_vertices)
                    .with("source_depth", source_depth);
                let receipt = image.output_receipt();
                drop(image);
                context.release_materialized_output(receipt)?;
                result.into()
            }
            GeoRequest::TransformBatch { name, points, grid } => {
                self.admit_output(points.len())?;
                let binding = self.profile.operation(name)?;
                let mut context = invocation.context(
                    budget.worker_context(purrdf_geo_kernel::binding::standard_reference())?,
                );
                let mut output = batch_output(points.len(), &mut context)?;
                binding
                    .chain()
                    .apply_batch_with_grid(points, &mut output, *grid, &mut context)?;
                let unit = binding
                    .chain()
                    .operations()
                    .last()
                    .expect("compiled nonempty chain")
                    .target()
                    .unit;
                output::admit_context(&mut context, output::batch(&output, output::transform)?)?;
                Value::Array(
                    output
                        .iter()
                        .map(|answer| {
                            encode::transform(
                                answer
                                    .as_ref()
                                    .expect("successful batch fills every result"),
                                unit,
                            )
                        })
                        .collect::<Result<_, _>>()?,
                )
            }
            GeoRequest::Profile => {
                output::admit(*budget, output::identity()?)?;
                encode::identity(&identity)
            }
            GeoRequest::Distance { crs, a, b, proof } => {
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                if *proof {
                    let (answer, receipt) =
                        prepared.distance_with_proof(&a.0, &b.0, &mut context)?;
                    output::admit_context(
                        &mut context,
                        output::metric(&answer)?
                            .with_child(output::proof(&receipt)?)
                            .ok_or(GeoError::ArithmeticOverflow("distance proof output"))?,
                    )?;
                    Object::new()
                        .with("estimate", encode::metric(&answer))
                        .with("proof", encode::proof(&receipt))
                        .into()
                } else {
                    let answer = prepared.distance(&a.0, &b.0, &mut context)?;
                    output::admit_context(&mut context, output::metric(&answer)?)?;
                    encode::metric(&answer)
                }
            }
            GeoRequest::DistanceBatch { crs, pairs } => {
                self.admit_output(pairs.len())?;
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                let mut output = batch_output(pairs.len(), &mut context)?;
                prepared.distance_batch_borrowed(
                    pairs.iter().map(|(a, b)| (&a.0, &b.0)),
                    &mut output,
                    &mut context,
                )?;
                output::admit_context(&mut context, output::batch(&output, output::metric)?)?;
                Value::Array(
                    output
                        .iter()
                        .map(|answer| {
                            encode::metric(
                                answer
                                    .as_ref()
                                    .expect("successful batch fills every result"),
                            )
                        })
                        .collect(),
                )
            }
            GeoRequest::Inverse { crs, a, b, proof } => {
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                if *proof {
                    let (answer, receipt) =
                        prepared.inverse_with_proof(&a.0, &b.0, &mut context)?;
                    output::admit_context(
                        &mut context,
                        output::inverse(&answer)?
                            .with_child(output::inverse_proof(&receipt)?)
                            .ok_or(GeoError::ArithmeticOverflow("inverse proof output"))?,
                    )?;
                    Object::new()
                        .with("inverse", encode::inverse(&answer))
                        .with("proof", encode::inverse_proof(&receipt))
                        .into()
                } else {
                    let answer = prepared.inverse(&a.0, &b.0, &mut context)?;
                    output::admit_context(&mut context, output::inverse(&answer)?)?;
                    encode::inverse(&answer)
                }
            }
            GeoRequest::InverseBatch { crs, pairs } => {
                self.admit_output(pairs.len())?;
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                let mut output = batch_output(pairs.len(), &mut context)?;
                prepared.inverse_batch_borrowed(
                    pairs.iter().map(|(a, b)| (&a.0, &b.0)),
                    &mut output,
                    &mut context,
                )?;
                output::admit_context(&mut context, output::batch(&output, output::inverse)?)?;
                Value::Array(
                    output
                        .iter()
                        .map(|answer| {
                            encode::inverse(
                                answer
                                    .as_ref()
                                    .expect("successful batch fills every result"),
                            )
                        })
                        .collect(),
                )
            }
            GeoRequest::Within {
                crs,
                a,
                b,
                threshold,
            } => {
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                // The standalone API and SPARQL compare the same promoted doubles.
                let inside = if threshold.is_negative() {
                    false
                } else {
                    prepared
                        .distance(&a.0, &b.0, &mut context)?
                        .within_reported(*threshold)
                };
                output::admit_context(&mut context, json::OutputLayout::scalar())?;
                Value::Bool(inside)
            }
            GeoRequest::WithinPhysical {
                crs,
                a,
                b,
                threshold,
            } => {
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                let inside = prepared.within_physical(&a.0, &b.0, threshold, &mut context)?;
                output::admit_context(&mut context, json::OutputLayout::scalar())?;
                Value::Bool(inside)
            }
            GeoRequest::Direct {
                crs,
                start,
                azimuth,
                length,
                grid,
            } => {
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                let answer =
                    prepared.direct_with_grid(&start.0, azimuth, length, *grid, &mut context)?;
                output::admit_context(&mut context, output::direct(&answer)?)?;
                encode::direct(&answer)
            }
            GeoRequest::DirectBatch { crs, inputs, grid } => {
                self.admit_output(inputs.len())?;
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                let mut output = batch_output(inputs.len(), &mut context)?;
                prepared.direct_batch_borrowed(
                    inputs
                        .iter()
                        .map(|(start, azimuth, length)| (&start.0, azimuth, length)),
                    &mut output,
                    *grid,
                    &mut context,
                )?;
                output::admit_context(&mut context, output::batch(&output, output::direct)?)?;
                Value::Array(
                    output
                        .iter()
                        .map(|answer| {
                            encode::direct(
                                answer
                                    .as_ref()
                                    .expect("successful batch fills every result"),
                            )
                        })
                        .collect(),
                )
            }
            GeoRequest::ShortestArcAt {
                crs,
                a,
                b,
                parameter,
            } => {
                let (prepared, mut context) = self.context(crs, *budget, invocation)?;
                let answer = prepared.shortest_arc_at(&a.0, &b.0, parameter, &mut context)?;
                output::admit_context(&mut context, output::direct(&answer)?)?;
                encode::direct(&answer)
            }
            GeoRequest::Geometry {
                function,
                arguments,
            } => {
                self.admit_output(arguments.len())?;
                function.check_argument_count(arguments.len())?;
                let count = arguments.len() as u64;
                let bytes = count
                    .checked_mul(size_of::<&purrdf_core::TermValue>() as u64)
                    .ok_or(GeoError::ArithmeticOverflow("borrowed geometry arguments"))?;
                budget.retain(count, bytes)?;
                let mut args: purrdf_core::SmallVec<[&purrdf_core::TermValue; 4]> =
                    purrdf_core::SmallVec::default();
                args.try_reserve_exact(arguments.len())
                    .map_err(|_| GeoError::MemoryExhausted {
                        limit: budget.policy().limits().max_workspace_bytes,
                    })?;
                args.extend(arguments);
                let mut work = invocation.phase(*budget);
                let term = purrdf_sparql_eval::geo::functions::compute_standard_with_preparation(
                    *function,
                    &self.profile,
                    &args,
                    &mut work,
                    *budget,
                )?;
                budget.retain(work.work, work.workspace)?;
                output::admit(*budget, output::term(&term)?)?;
                encode::term(&term)?
            }
            GeoRequest::GeometryMetric { metric, geometry } => {
                let preparation = geometry.prepare_in_policy_observed(
                    &self.profile,
                    budget.remaining()?,
                    invocation,
                )?;
                budget.retain(preparation.work_items, preparation.workspace_bytes)?;
                let mut context =
                    invocation.context(budget.worker_context(preparation.geometry.reference())?);
                let prepared = &preparation.geometry;
                let estimate = match metric {
                    purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::Length => {
                        purrdf_geo_kernel::ellipsoidal::length(prepared, &mut context)?
                    }
                    purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::Perimeter => {
                        purrdf_geo_kernel::ellipsoidal::perimeter(prepared, &mut context)?
                    }
                    purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::Area => {
                        purrdf_geo_kernel::ellipsoidal::area(prepared, &mut context)?
                    }
                    purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::GeodesicAreaIntegral => {
                        let [curve] = prepared.curves() else {
                            return Err(GeoError::domain(
                                "a geodesic area integral requires one selected original edge",
                            )
                            .into());
                        };
                        let [edge] = curve.edges() else {
                            return Err(GeoError::domain(
                                "a geodesic area integral requires one selected original edge",
                            )
                            .into());
                        };
                        if !prepared.points().is_empty()
                            || !prepared.symbolic_points().is_empty()
                            || !matches!(
                                prepared.region(),
                                purrdf_geo_kernel::PreparedRegion::Empty
                            )
                        {
                            return Err(GeoError::domain("a geodesic area integral requires an edge without additional geometry").into());
                        }
                        purrdf_geo_kernel::ellipsoidal::geodesic_area_integral(edge, &mut context)?
                    }
                    purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::Distance
                    | purrdf_geo_kernel::ellipsoidal::GeometryMetricLaw::AreaIntegral => {
                        return Err(GeoError::domain(
                            "a general distance needs two prepared geometries",
                        )
                        .into());
                    }
                };
                output::admit_context(&mut context, output::geometry_metric(&estimate)?)?;
                encode::geometry_metric(&estimate)
            }
            GeoRequest::OffsetContains {
                crs,
                geometry,
                radius,
                point,
            } => {
                let preparation = geometry.prepare_in_policy_observed(
                    &self.profile,
                    budget.remaining()?,
                    invocation,
                )?;
                budget.retain(preparation.work_items, preparation.workspace_bytes)?;
                let point_reference = self.profile.reference(crs)?;
                if point_reference != preparation.geometry.reference() {
                    return Err(GeoError::MissingOperation {
                        source: point_reference.id().digest().to_string(),
                        target: preparation.geometry.reference().id().digest().to_string(),
                    }
                    .into());
                }
                let offset = prepare_offset(preparation.geometry, radius, &mut budget, invocation)?;
                let mut context =
                    invocation.context(budget.worker_context(offset.source().reference())?);
                let source_receipt = offset.source().source_receipt();
                let inside = offset.contains_prepared(&point.0, &source_receipt, &mut context)?;
                output::admit_context(&mut context, output::offset(radius.exact())?)?;
                Object::new()
                    .with("inside", inside)
                    .with("offset", offset.id().to_string())
                    .with("source", offset.source().id().to_string())
                    .with(
                        "radius_metres",
                        super::profile::exact_decimal(radius.exact())?,
                    )
                    .into()
            }
            GeoRequest::BufferPoints {
                crs,
                geometry,
                radius,
            }
            | GeoRequest::Buffer {
                crs,
                geometry,
                radius,
            } => {
                let preparation = geometry.prepare_in_policy_observed(
                    &self.profile,
                    budget.remaining()?,
                    invocation,
                )?;
                budget.retain(preparation.work_items, preparation.workspace_bytes)?;
                let offset = prepare_offset(preparation.geometry, radius, &mut budget, invocation)?;
                let mut context =
                    invocation.context(budget.worker_context(offset.source().reference())?);
                let receipt = offset.source().source_receipt();
                let materialized = if matches!(request, GeoRequest::BufferPoints { .. }) {
                    offset.materialize_points_prepared(
                        &self.profile,
                        crs,
                        &receipt,
                        &mut context,
                    )?
                } else {
                    offset.materialize_prepared(&self.profile, crs, &receipt, &mut context)?
                };
                let lexical = if radius.exact().is_zero() {
                    purrdf_geo_kernel::wkt::write_exact_in_context(
                        materialized.literal(),
                        &mut context,
                    )?
                } else {
                    purrdf_geo_kernel::wkt::write_in_context(
                        materialized.literal(),
                        purrdf_geo_kernel::operation::CoordinateUnit::Degrees
                            .output_decimal_places(0),
                        &mut context,
                    )?
                };
                let geometry = purrdf_geo_kernel::carrier::term_in_context(
                    lexical,
                    purrdf_geo_kernel::carrier::CarrierFormat::Wkt,
                    &mut context,
                )?;
                output::admit_context(
                    &mut context,
                    output::record(6, 1024)?
                        .with_child(output::term(&geometry)?)
                        .ok_or(GeoError::ArithmeticOverflow("buffer output layout"))?,
                )?;
                let result = Object::new()
                    .with("geometry", encode::term(&geometry)?)
                    .with("offset", offset.id().to_string())
                    .with("source", materialized.source_id().to_string())
                    .with("law", materialized.law_id().digest().to_string())
                    .with(
                        "outward_error_metres",
                        super::profile::exact_decimal(materialized.outward_error().exact())?,
                    )
                    .with(
                        "certificate",
                        purrdf_hash::hex::encode(&materialized.certificate_bytes()),
                    );
                let receipt = materialized.output_receipt();
                drop(materialized);
                context.release_materialized_output(receipt)?;
                result.into()
            }
            GeoRequest::GeometryDistance { a, b } => {
                let (a, b, mut context) =
                    super::geometry::prepare_pair(a, b, &self.profile, *budget, invocation)?;
                let estimate = purrdf_geo_kernel::ellipsoidal::distance(
                    &a.geometry,
                    &b.geometry,
                    &mut context,
                )?;
                output::admit_context(&mut context, output::geometry_metric(&estimate)?)?;
                encode::geometry_metric(&estimate)
            }
            GeoRequest::GeometryRelate { a, b } => {
                let (a, b, mut context) =
                    super::geometry::prepare_pair(a, b, &self.profile, *budget, invocation)?;
                let matrix = purrdf_geo_kernel::atlas::relate_prepared(
                    &a.geometry,
                    &b.geometry,
                    [&a.geometry.source_receipt(), &b.geometry.source_receipt()],
                    &mut context,
                )?;
                output::admit_context(&mut context, output::record(5, 9 + 4 * 64)?)?;
                Object::new()
                    .with("matrix", matrix.to_string())
                    .with(
                        "law",
                        purrdf_geo_kernel::atlas::topology_law_id()
                            .digest()
                            .to_string(),
                    )
                    .with("binding", a.geometry.reference().id().digest().to_string())
                    .with("source_a", a.geometry.id().to_string())
                    .with("source_b", b.geometry.id().to_string())
                    .into()
            }
            GeoRequest::Cell { grid, point, level } => {
                let layout = output::cell()?;
                output::reserve(&mut budget, layout)?;
                encode::cell(CubeHilbertQ62V1::new(*grid).assign_in_policy(
                    &point.0,
                    *level,
                    budget.remaining()?,
                )?)
            }
            GeoRequest::CellBatch {
                grid,
                points,
                level,
            } => {
                self.admit_output(points.len())?;
                let layout = output::array(points.len(), output::cell()?)?;
                output::reserve(&mut budget, layout)?;
                let grid = CubeHilbertQ62V1::new(*grid);
                let count = points.len() as u64;
                let bytes = count
                    .checked_mul(size_of::<CellId>() as u64)
                    .ok_or(GeoError::ArithmeticOverflow("cell batch output storage"))?;
                budget.retain(count, bytes)?;
                let policy = budget.remaining()?;
                let mut output = vec![CellId::root(grid.profile_id(), 0)?; points.len()];
                grid.assign_batch_in_policy(
                    points.iter().map(|point| &point.0),
                    *level,
                    &mut output,
                    policy,
                )?;
                Value::Array(output.into_iter().map(encode::cell).collect())
            }
            GeoRequest::CellHierarchy {
                cell,
                ancestor,
                stored_level,
            } => {
                output::admit(
                    *budget,
                    output::record(5, 0)?
                        .with_child(output::array(7, output::cell()?)?)
                        .and_then(|layout| layout.with_child(output::range().ok()?))
                        .ok_or(GeoError::ArithmeticOverflow("cell hierarchy output layout"))?,
                )?;
                let range = cell.descendant_range(*stored_level)?;
                let parent = if cell.level() == 0 {
                    Value::Null
                } else {
                    encode::cell(cell.parent()?)
                };
                let children = if cell.level() == 30 {
                    Value::Array(Vec::new())
                } else {
                    Value::Array(cell.children()?.into_iter().map(encode::cell).collect())
                };
                let ancestor = ancestor
                    .map(|level| cell.ancestor(level).map(encode::cell))
                    .transpose()?
                    .unwrap_or(Value::Null);
                Object::new()
                    .with("cell", encode::cell(*cell))
                    .with("parent", parent)
                    .with("children", children)
                    .with("ancestor", ancestor)
                    .with(
                        "descendant_range",
                        Object::new()
                            .with("profile", range.profile().digest().to_string())
                            .with("min", encode::key(range.min()))
                            .with("max", encode::key(range.max()))
                            .with("stride", encode::key(range.stride()))
                            .with("stored_level", range.stored_level())
                            .with("logical_count", range.count().to_string()),
                    )
                    .into()
            }
            GeoRequest::CellScale {
                grid,
                level,
                maximum_edge,
            } => {
                let grid = CubeHilbertQ62V1::new(*grid);
                let mut work = invocation.phase(*budget);
                let (bounds, selected_level) = grid
                    .physical_scale_bounds_and_level_in_policy_metered(
                        *level,
                        maximum_edge.as_ref(),
                        budget.remaining()?,
                        &mut work,
                    )?;
                work.retain_phase(&mut budget, bounds.retained_workspace_bytes())?;
                output::admit(*budget, output::cell_scale(&bounds)?)?;
                Object::new()
                    .with("profile", grid.profile_id().digest().to_string())
                    .with("level", *level)
                    .with(
                        "lower_metres",
                        encode::lower_decimal(bounds.lower().exact()),
                    )
                    .with(
                        "upper_metres",
                        encode::upper_decimal(bounds.upper().exact()),
                    )
                    .with(
                        "nominal_lower_metres",
                        encode::lower_decimal(bounds.nominal_lower().exact()),
                    )
                    .with(
                        "nominal_upper_metres",
                        encode::upper_decimal(bounds.nominal_upper().exact()),
                    )
                    .with(
                        "footprint_guard_metres",
                        encode::upper_decimal(bounds.footprint_guard().exact()),
                    )
                    .with("selected_level", selected_level)
                    .into()
            }
        };
        Ok(result)
    }

    /// Strict JSON request to canonical version-one success/refusal response.
    ///
    /// Admitted sessions retain the same effective identity on both outcomes.
    /// A constructor refusal retained by `new` carries null identity because no
    /// identity was compiled. It never has a result; every host carries these bytes.
    #[must_use]
    pub fn call_string(&self, text: &str) -> String {
        self.response(text).0
    }

    /// Response bytes plus success status for hosts with process/error channels.
    #[must_use]
    pub fn response(&self, text: &str) -> (String, bool) {
        let identity = self.identity();
        let invocation = Invocation::new(self.profile.policy());
        let answer = (|| {
            identity.clone()?;
            let (value, budget) =
                super::read_record_for_invocation(text, self.profile.policy(), &invocation)?;
            invocation.budget(budget);
            match value.get("operation").and_then(Value::as_str) {
                Some("point-index") => self
                    .point_index_value(&value, budget, &invocation)
                    .map(|(index, _)| index.metadata()),
                Some("point-index-query") => {
                    let mut fields =
                        json::record::Record::new(&value, "point index query version one")?;
                    if fields.required::<u32>("version")? != 1 {
                        return Err(json::record::DecodeError::custom(
                            "geographic request version must be 1",
                        )
                        .into());
                    }
                    fields.tag("operation", &["point-index-query"])?;
                    let (index, budget) = fields.required_with("index", |value| {
                        self.point_index_value(value, budget, &invocation)
                    })?;
                    let result = fields.required_with("search", |value| {
                        index.call_value(value, budget, &invocation)
                    })?;
                    fields.deny_unknown()?;
                    Ok(result)
                }
                _ => GeoRequest::from_value(&value)
                    .and_then(|request| self.call_with_preparation(&request, budget, &invocation)),
            }
        })();
        let success = answer.is_ok();
        let fields = Object::new()
            .with("version", 1_u32)
            .with("identity", identity.ok().as_ref().map(encode::identity));
        let fields = match answer {
            Ok(result) => fields.with("result", result),
            Err(error) => fields.with("error", encode::refusal_admitted(&error, &invocation)),
        };
        (json::write_compact(&fields.into()), success)
    }

    fn context<'a>(
        &self,
        crs: &purrdf_geo_kernel::Crs,
        mut budget: PreparationBudget,
        invocation: &'a Invocation,
    ) -> Result<(PreparedGeodesic, super::invocation::Context<'a>), GeoError> {
        let reference = self.profile.reference(crs)?;
        // Only preparation and shared-table lookup hold the lock. Numerical calls
        // retain immutable Arc tables and independent worker scratch afterward.
        // Failed preparation is not retained: a worker's environment cannot poison
        // another worker's successful use of this session.
        let mut cache = self
            .prepared
            .lock()
            .map_err(|_| GeoError::config("geographic preparation cache was poisoned"))?;
        budget.retain(0, cache.retained_bytes)?;
        let mut context = invocation.context(budget.worker_context(reference)?);
        let prepared = if let Some(entry) = cache
            .entries
            .iter()
            .find(|entry| entry.geodesic.reference() == reference)
        {
            context.set_prepared_arithmetic(entry.arithmetic.clone());
            entry.geodesic.clone()
        } else {
            let prepared_reference = context.clone_reference_for_preparation()?;
            let geodesic = PreparedGeodesic::prepare(prepared_reference, &mut context)?;
            let preparation_work = context.work_items();
            context.set_preparation_work(preparation_work)?;
            let arithmetic = context
                .prepared_arithmetic()
                .ok_or(GeoError::ArithmeticOverflow(
                    "prepared session arithmetic invariant",
                ))?;
            let dynamic_reference = geodesic.reference().ellipsoid().retained_limb_bytes();
            let bytes = geodesic
                .retained_workspace_bytes()
                .checked_add(arithmetic.workspace_bytes() as u64)
                .and_then(|bytes| bytes.checked_add(dynamic_reference))
                .and_then(|bytes| bytes.checked_add(size_of::<SessionPreparation>() as u64 + 64))
                .and_then(|bytes| bytes.checked_add(cache.retained_bytes))
                .ok_or(GeoError::MemoryExhausted {
                    limit: self.profile.policy().limits().max_workspace_bytes,
                })?;
            let additional =
                bytes
                    .checked_sub(cache.retained_bytes)
                    .ok_or(GeoError::ArithmeticOverflow(
                        "session preparation cache receipt",
                    ))?;
            // The preparing worker still owns its integer destinations and
            // numerical caches. Retain the newly published immutable tables
            // alongside that live baseline instead of replacing it with the
            // smaller shared-cache receipt.
            let retained = context
                .retained_workspace_bytes()
                .checked_add(additional.checked_sub(dynamic_reference).ok_or(
                    GeoError::ArithmeticOverflow("prepared reference cache receipt"),
                )?)
                .ok_or(GeoError::MemoryExhausted {
                    limit: self.profile.policy().limits().max_workspace_bytes,
                })?;
            context.set_retained_workspace(retained)?;
            cache
                .entries
                .try_reserve_exact(1)
                .map_err(|_| GeoError::MemoryExhausted {
                    limit: self.profile.policy().limits().max_workspace_bytes,
                })?;
            cache.entries.push(SessionPreparation {
                geodesic: geodesic.clone(),
                arithmetic,
            });
            cache.retained_bytes = bytes;
            geodesic
        };
        drop(cache);
        Ok((prepared, context))
    }

    fn admit_output(&self, count: usize) -> Result<(), GeoError> {
        let limit = self.profile.policy().limits().max_output_elements;
        if u64::try_from(count).is_err() || count as u64 > limit {
            return Err(GeoError::OutputExhausted { limit });
        }
        Ok(())
    }
}

fn compile_identity(
    profile: &GeoProfile,
    budget: &mut PreparationBudget,
) -> Result<GeoQueryIdentity, GeoError> {
    admit_session_storage(budget)?;
    purrdf_geo_kernel::binding::standard_vocabulary_in_budget(budget)?;
    profile.query_identity_in_budget(budget)
}

// The profile object's reachable storage is admitted by its native binding
// home; this host owns only the two shared ownership headers and cache handle.
fn admit_session_storage(budget: &mut PreparationBudget) -> Result<(), GeoError> {
    budget.retain(
        1,
        (size_of::<Mutex<SessionPreparations>>() + 4 * size_of::<usize>()) as u64,
    )
}

// Caller-owned slots are part of the same retained invocation storage. Admit
// the complete allocation and initialization before Vec construction; native
// batch entry keeps these actual preparation counts instead of resetting them.
fn batch_output<T>(count: usize, context: &mut MetricContext) -> Result<Vec<Option<T>>, GeoError> {
    let bytes = (count as u64)
        .checked_mul(size_of::<Option<T>>() as u64)
        .ok_or(GeoError::ArithmeticOverflow("batch output storage"))?;
    let retained = context
        .retained_workspace_bytes()
        .checked_add(bytes)
        .ok_or(GeoError::ArithmeticOverflow("batch retained storage"))?;
    context.set_retained_workspace(retained)?;
    context.charge_work(count as u64)?;
    context.set_preparation_work(context.work_items())?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })?;
    output.resize_with(count, || None);
    Ok(output)
}

// Constructor storage includes the existing immutable source. The invocation
// already retains that graph, so admit the constructor against the same source
// allowance and retain only its additional radius/identity storage afterwards.
fn prepare_offset(
    geometry: purrdf_geo_kernel::PreparedGeometry,
    radius: &purrdf_geo_kernel::Metres,
    budget: &mut PreparationBudget,
    invocation: &Invocation,
) -> Result<purrdf_geo_kernel::OffsetRegion, GeoCallError> {
    let mut phase = invocation.phase(*budget);
    budget
        .prepare_offset(geometry, radius, &mut phase)
        .map_err(GeoCallError::from)
}
