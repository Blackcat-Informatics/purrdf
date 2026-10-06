// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original carrier or explicit prepared-curve records over native constructors.

use purrdf_core::TermValue;
use purrdf_geo_kernel::{
    AzimuthLengthArc, Coord, Crs, ExecutionPolicy, GeoError, GeoProfile, Metres, MetricContext,
    OrientedInterior, PreparedCoordinate, PreparedCurve, PreparedEdge, PreparedGeometry,
    PreparedPolygon, PreparedRegion, Rat, RegionInterior, ShortestGeodesicArc, SourceLinearEdge,
};
use purrdf_lex::json::{
    Value,
    record::{Record, items_with},
};

use super::{GeoCallError, invocation::Invocation, profile::decimal, request};

/// Original geometry input, with an explicit edge law when no carrier is used.
#[derive(Clone, Debug)]
pub enum GeometryInput {
    /// Original RDF geometry literal, retaining exact written carrier coordinates.
    Carrier(TermValue),
    /// Caller-selected native points, curves and region; no ring is invented.
    Prepared(Box<GeometryParts>),
    /// Complete continuous image under an explicitly named registered chain.
    Image {
        /// Exact operation dispatch IRI; its declared target is geographic.
        operation: Crs,
        /// Original source carrier, retained without angular materialization.
        geometry: TermValue,
        /// Actual observation epoch when a rate-bearing model needs it.
        epoch: Option<Rat>,
    },
}

/// Explicit source records for one immutable prepared geometry.
#[derive(Clone, Debug)]
pub struct GeometryParts {
    /// Exact declared geographic source-reference IRI and axis interpretation.
    pub crs: Crs,
    /// Original point coordinates, including optional Z and M ordinates.
    pub points: Vec<Coord>,
    /// Each curve retains its written ordered edges and their selected laws.
    pub curves: Vec<Vec<EdgeInput>>,
    /// The complete explicitly selected surface region.
    pub region: RegionInput,
}

/// Original source ordinates and exact curve branch parameters.
#[derive(Clone, Debug)]
pub enum EdgeInput {
    /// Linear interpolation of the complete written source coordinates.
    SourceLinear(Coord, Coord),
    /// Exact endpoints select a certified unique shortest ellipsoidal branch.
    ShortestGeodesic(Coord, Coord),
    /// Clockwise azimuth and actual propagation length select the geodesic branch.
    AzimuthLength(Coord, Rat, Metres),
    /// Complete continuous image of the original source-linear endpoints.
    Image {
        /// Explicit named chain; its actual target must equal this geometry's reference.
        operation: Crs,
        /// Original source axes and complete Z/M ordinates.
        start: Coord,
        /// Original source axes and complete Z/M ordinates.
        end: Coord,
        /// Actual observation epoch for the selected operation.
        epoch: Option<Rat>,
    },
}

/// A finite explicitly selected prepared surface region.
#[derive(Clone, Debug)]
pub enum RegionInput {
    /// No surface interior.
    Empty,
    /// The complete identified surface.
    Whole,
    /// The complete carrier polygon union, including original holes.
    Carrier(TermValue),
    /// The complement of the complete carrier polygon union.
    Complement(TermValue),
    /// Native original-law polygon union, optionally selecting its complete complement.
    Native {
        /// Every polygon selects the intersection of its declared ring sides.
        polygons: Vec<PolygonInput>,
        /// Complement the complete union after each polygon's own selection.
        complement: bool,
    },
}

/// Explicit ring-side and source-law records for one native prepared polygon.
#[derive(Clone, Debug)]
pub struct PolygonInput {
    /// Original ordered rings, each preserving its selected edge laws.
    pub rings: Vec<Vec<EdgeInput>>,
    /// Physical left intersection, or the complement of that intersection.
    pub side: OrientedInterior,
    /// Additional explicit selection over the ring-side base.
    pub interior: RegionInterior,
}

impl RegionInput {
    fn native_rings(&self) -> impl Iterator<Item = &Vec<EdgeInput>> {
        let polygons = match self {
            Self::Native { polygons, .. } => Some(polygons),
            _ => None,
        };
        polygons
            .into_iter()
            .flatten()
            .flat_map(|polygon| &polygon.rings)
    }
}

pub(super) struct PreparedInput {
    pub(super) geometry: PreparedGeometry,
    pub(super) work_items: u64,
    pub(super) workspace_bytes: u64,
}

/// Prepare both source graphs under one cumulative input and worker admission.
pub(super) fn prepare_pair<'a>(
    a: &GeometryInput,
    b: &GeometryInput,
    profile: &GeoProfile,
    budget: purrdf_geo_kernel::PreparationBudget,
    invocation: &'a Invocation,
) -> Result<(PreparedInput, PreparedInput, super::invocation::Context<'a>), GeoCallError> {
    let mut budget = invocation.track_budget(budget);
    let a = a.prepare_in_policy_observed(profile, budget.remaining()?, invocation)?;
    budget.retain(a.work_items, a.workspace_bytes)?;
    let b = b.prepare_in_policy_observed(profile, budget.remaining()?, invocation)?;
    budget.retain(b.work_items, b.workspace_bytes)?;
    let context = invocation.context(budget.worker_context(a.geometry.reference())?);
    Ok((a, b, context))
}

impl GeometryInput {
    /// Strict record decoder; original values remain exact before preparation.
    /// # Errors
    /// Refuses duplicate/unknown fields, malformed coordinates and edge-law tags.
    pub fn from_value(value: &Value) -> Result<Self, GeoCallError> {
        if value.get("kind").and_then(Value::as_str) == Some("image") {
            let mut fields = Record::new(value, "original continuous operation image")?;
            fields.tag("kind", &["image"])?;
            let operation: String = fields.required("operation")?;
            let result = Self::Image {
                operation: Crs::new(operation)?,
                geometry: fields.required_with("geometry", request::term)?,
                epoch: fields.optional_with("epoch_decimal_year", decimal)?,
            };
            fields.deny_unknown()?;
            return Ok(result);
        }
        if value.get("kind").and_then(Value::as_str) != Some("prepared") {
            return request::term(value).map(Self::Carrier).map_err(Into::into);
        }
        let mut fields = Record::new(value, "explicit prepared geographic geometry")?;
        fields.tag("kind", &["prepared"])?;
        let crs = request::crs(&mut fields)?;
        let points = fields
            .optional_with("points", |value| items_with(value, coordinate))?
            .unwrap_or_default();
        let curves = fields
            .optional_with("curves", |value| {
                items_with(value, |value| items_with(value, edge))
            })?
            .unwrap_or_default();
        let region = fields
            .optional_with("region", region)?
            .unwrap_or(RegionInput::Empty);
        fields.deny_unknown()?;
        Ok(Self::Prepared(Box::new(GeometryParts {
            crs,
            points,
            curves,
            region,
        })))
    }

    /// Prepare with the kernel's one reference, interpolation and identity laws.
    /// # Errors
    /// Propagates invalid reference, coordinate, branch and complete admission errors.
    pub fn prepare(&self, profile: &GeoProfile) -> Result<PreparedGeometry, GeoCallError> {
        self.prepare_with_admission(profile)
            .map(|prepared| prepared.geometry)
    }

    pub(super) fn prepare_with_admission(
        &self,
        profile: &GeoProfile,
    ) -> Result<PreparedInput, GeoCallError> {
        self.prepare_in_policy(profile, profile.policy())
    }

    pub(super) fn prepare_in_policy(
        &self,
        profile: &GeoProfile,
        policy: ExecutionPolicy,
    ) -> Result<PreparedInput, GeoCallError> {
        let invocation = Invocation::new(policy);
        self.prepare_in_policy_observed(profile, policy, &invocation)
    }

    pub(super) fn prepare_in_policy_observed(
        &self,
        profile: &GeoProfile,
        policy: ExecutionPolicy,
        invocation: &Invocation,
    ) -> Result<PreparedInput, GeoCallError> {
        match self {
            Self::Carrier(term) => prepare_carrier(term, profile, policy, invocation),
            Self::Prepared(parts) => parts.prepare(profile, policy, invocation),
            Self::Image {
                operation,
                geometry,
                epoch,
            } => prepare_image(
                operation,
                geometry,
                epoch.as_ref(),
                profile,
                policy,
                invocation,
            ),
        }
    }
}

fn prepare_image(
    operation: &Crs,
    term: &TermValue,
    epoch: Option<&Rat>,
    profile: &GeoProfile,
    policy: ExecutionPolicy,
    invocation: &Invocation,
) -> Result<PreparedInput, GeoCallError> {
    let mut budget = invocation.track_budget(purrdf_geo_kernel::PreparationBudget::new(policy));
    let (work, storage) = request::reference_allowance(operation)?;
    budget.retain(work, storage)?;
    let binding = profile.operation(operation)?;
    let reference = profile.reference(binding.target())?.clone();
    let mut phase = invocation.phase(*budget);
    let parsed = purrdf_geo_kernel::carrier::geometry_arg_metered(
        purrdf_geo_kernel::standard_vocabulary(),
        term,
        budget.remaining()?,
        &mut phase,
    )?;
    budget.retain(
        parsed.receipt().work_items(),
        parsed.receipt().workspace_bytes(),
    )?;
    let mut context = invocation.context(MetricContext::new(reference, policy)?);
    budget.install(&mut context)?;
    let geometry = profile.prepare_transformed_literal_named(
        operation,
        parsed.literal(),
        epoch,
        &mut context,
    )?;
    budget.retain(
        geometry.preparation_work_items(),
        geometry.retained_workspace_bytes(),
    )?;
    Ok(PreparedInput {
        geometry,
        work_items: budget.work_items(),
        workspace_bytes: budget.workspace_bytes(),
    })
}

fn prepare_carrier(
    term: &TermValue,
    profile: &GeoProfile,
    policy: ExecutionPolicy,
    invocation: &Invocation,
) -> Result<PreparedInput, GeoCallError> {
    let mut budget = invocation.track_budget(purrdf_geo_kernel::PreparationBudget::new(policy));
    let mut phase = invocation.phase(*budget);
    let parsed = purrdf_geo_kernel::carrier::geometry_arg_metered(
        purrdf_geo_kernel::standard_vocabulary(),
        term,
        budget.remaining()?,
        &mut phase,
    )?;
    budget.retain(
        parsed.receipt().work_items(),
        parsed.receipt().workspace_bytes(),
    )?;
    let mut phase = invocation.phase(*budget);
    let geometry = PreparedGeometry::from_literal_in_policy_metered(
        parsed.literal(),
        profile,
        budget.remaining()?,
        &mut phase,
    )?;
    budget.retain(
        geometry.preparation_work_items(),
        geometry.retained_workspace_bytes(),
    )?;
    Ok(PreparedInput {
        geometry,
        work_items: budget.work_items(),
        workspace_bytes: budget.workspace_bytes(),
    })
}

impl GeometryParts {
    fn prepare(
        &self,
        profile: &GeoProfile,
        policy: ExecutionPolicy,
        invocation: &Invocation,
    ) -> Result<PreparedInput, GeoCallError> {
        let limit = policy.limits();
        let mut budget = invocation.track_budget(purrdf_geo_kernel::PreparationBudget::new(policy));
        let (reference_work, reference_storage) = request::reference_allowance(&self.crs)?;
        budget.retain(reference_work, reference_storage)?;
        let reference = profile.reference(&self.crs)?.clone();
        let polygons = match &self.region {
            RegionInput::Native { polygons, .. } => polygons.as_slice(),
            _ => &[],
        };
        // Empty containers still own capacity and require an admitted visit.
        // The source records and the new native arrays coexist during preparation.
        budget.retain(
            1_u64
                .saturating_add(self.curves.len() as u64)
                .saturating_add(polygons.len() as u64),
            0,
        )?;
        let mut source_containers = (self.points.capacity() as u64)
            .saturating_mul(size_of::<Coord>() as u64)
            .saturating_add(
                (self.curves.capacity() as u64).saturating_mul(size_of::<Vec<EdgeInput>>() as u64),
            );
        let mut output_containers = (self.points.len() as u64)
            .saturating_mul(size_of::<PreparedCoordinate>() as u64)
            .saturating_add(
                (self.curves.len() as u64).saturating_mul(size_of::<PreparedCurve>() as u64),
            );
        if let RegionInput::Native { polygons, .. } = &self.region {
            source_containers = source_containers.saturating_add(
                (polygons.capacity() as u64).saturating_mul(size_of::<PolygonInput>() as u64),
            );
            output_containers = output_containers.saturating_add(
                (polygons.len() as u64).saturating_mul(size_of::<PreparedPolygon>() as u64),
            );
            for polygon in polygons {
                budget.retain(polygon.rings.len() as u64, 0)?;
                let source_bytes =
                    (polygon.rings.capacity() as u64)
                        .saturating_mul(size_of::<Vec<EdgeInput>>() as u64);
                let output_bytes =
                    (polygon.rings.len() as u64).saturating_mul(size_of::<PreparedCurve>() as u64);
                source_containers = source_containers.saturating_add(source_bytes);
                output_containers = output_containers.saturating_add(output_bytes);
            }
        }
        for curve in self.curves.iter().chain(self.region.native_rings()) {
            source_containers = source_containers.saturating_add(
                (curve.capacity() as u64).saturating_mul(size_of::<EdgeInput>() as u64),
            );
            output_containers =
                output_containers.saturating_add((curve.len() as u64).saturating_mul(size_of::<
                    PreparedEdge,
                >(
                )
                    as u64));
        }
        budget.retain(0, source_containers.saturating_add(output_containers))?;
        let edges = self
            .curves
            .iter()
            .chain(self.region.native_rings())
            .try_fold(0_u64, |count, curve| count.checked_add(curve.len() as u64))
            .ok_or(GeoError::ArithmeticOverflow("prepared input edge count"))?;
        let vertices = edges
            .checked_mul(2)
            .and_then(|count| count.checked_add(self.points.len() as u64))
            .ok_or(GeoError::ArithmeticOverflow(
                "prepared input coordinate count",
            ))?;
        if vertices > limit.max_output_elements {
            return Err(GeoError::OutputExhausted {
                limit: limit.max_output_elements,
            }
            .into());
        }
        if vertices > limit.max_work_items {
            return Err(GeoError::WorkExhausted {
                limit: limit.max_work_items,
            }
            .into());
        }
        budget.retain(vertices, 0)?;
        let coordinate_bytes = |coordinate: &Coord| {
            [
                Some(coordinate.x()),
                Some(coordinate.y()),
                coordinate.z(),
                coordinate.m(),
            ]
            .into_iter()
            .flatten()
            .map(|value| {
                (value.numerator().allocated_bytes() as u64)
                    .saturating_add(value.denominator().allocated_bytes() as u64)
            })
            .fold(0_u64, u64::saturating_add)
        };
        let dynamic_source = self
            .points
            .iter()
            .map(coordinate_bytes)
            .chain(
                self.curves
                    .iter()
                    .chain(self.region.native_rings())
                    .flatten()
                    .map(|edge| match edge {
                        EdgeInput::SourceLinear(a, b) | EdgeInput::ShortestGeodesic(a, b) => {
                            coordinate_bytes(a).saturating_add(coordinate_bytes(b))
                        }
                        EdgeInput::AzimuthLength(start, azimuth, length) => coordinate_bytes(start)
                            .saturating_add(azimuth.numerator().allocated_bytes() as u64)
                            .saturating_add(azimuth.denominator().allocated_bytes() as u64)
                            .saturating_add(length.exact().numerator().allocated_bytes() as u64)
                            .saturating_add(length.exact().denominator().allocated_bytes() as u64),
                        EdgeInput::Image {
                            operation,
                            start,
                            end,
                            epoch,
                        } => coordinate_bytes(start)
                            .saturating_add(coordinate_bytes(end))
                            .saturating_add(operation.retained_text_bytes() as u64)
                            .saturating_add(
                                epoch
                                    .as_ref()
                                    .map_or(0, |value| value.allocated_bytes() as u64),
                            ),
                    }),
            )
            .fold(0_u64, u64::saturating_add);
        let dynamic = dynamic_source.saturating_mul(32);
        let retained = vertices
            .checked_mul(4096)
            .and_then(|count| count.checked_add(65_536))
            .and_then(|count| count.checked_add(dynamic))
            .ok_or(GeoError::MemoryExhausted {
                limit: limit.max_workspace_bytes,
            })?;
        if retained > limit.max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: limit.max_workspace_bytes,
            }
            .into());
        }
        // Retain the complete graph's conservative storage allowance while
        // admitting every actual exact copy and numerical phase cumulatively.
        budget.retain(0, retained)?;
        let mut points = Vec::with_capacity(self.points.len());
        for point in &self.points {
            points.push(prepare_coordinate(
                point,
                &reference,
                &mut budget,
                invocation,
            )?);
        }
        let curves = prepare_curves(&self.curves, &reference, profile, &mut budget, invocation)?;
        let (region, region_work, region_workspace) = match &self.region {
            RegionInput::Empty => (PreparedRegion::Empty, 0, 0),
            RegionInput::Whole => (PreparedRegion::Whole, 0, 0),
            RegionInput::Native {
                polygons,
                complement,
            } => {
                let mut prepared = Vec::with_capacity(polygons.len());
                for polygon in polygons {
                    let rings = prepare_curves(
                        &polygon.rings,
                        &reference,
                        profile,
                        &mut budget,
                        invocation,
                    )?;
                    let mut context = invocation
                        .context(MetricContext::new(reference.clone(), budget.remaining()?)?);
                    let polygon = PreparedPolygon::from_curves(rings, polygon.side, &mut context)?
                        .with_interior(polygon.interior);
                    budget.retain(context.work_items(), 0)?;
                    prepared.push(polygon);
                }
                let region = PreparedRegion::polygons(prepared);
                (
                    if *complement {
                        region.complement()
                    } else {
                        region
                    },
                    0,
                    0,
                )
            }
            RegionInput::Carrier(term) | RegionInput::Complement(term) => {
                let source = prepare_carrier(term, profile, budget.remaining()?, invocation)?;
                if source.geometry.reference() != &reference {
                    return Err(GeoError::MissingOperation {
                        source: source.geometry.reference().id().digest().to_string(),
                        target: reference.id().digest().to_string(),
                    }
                    .into());
                }
                let region = source.geometry.region().clone();
                let region = if matches!(&self.region, RegionInput::Complement(_)) {
                    region.complement()
                } else {
                    region
                };
                (region, source.work_items, source.workspace_bytes)
            }
        };
        let completed_work = budget.work_items();
        // Ownership of the already built graph moves into the kernel's final
        // complete-parts admission. Do not reserve the same graph twice.
        let mut budget = invocation.track_budget(purrdf_geo_kernel::PreparationBudget::new(policy));
        // Original typed records remain borrowed through evaluation. Only the
        // newly built native graph transfers to the final kernel admission.
        budget.retain(
            completed_work,
            source_containers
                .saturating_add(dynamic_source)
                .saturating_add(reference_storage),
        )?;
        budget.retain(region_work, region_workspace)?;
        let mut phase = invocation.phase(*budget);
        let geometry = PreparedGeometry::from_parts_metered(
            reference,
            points,
            curves,
            region,
            budget.remaining()?,
            &mut phase,
        )?;
        budget.retain(
            geometry.preparation_work_items(),
            geometry.retained_workspace_bytes(),
        )?;
        Ok(PreparedInput {
            geometry,
            work_items: budget.work_items(),
            workspace_bytes: budget.workspace_bytes(),
        })
    }
}

fn prepare_curves(
    sources: &[Vec<EdgeInput>],
    reference: &purrdf_geo_kernel::GeographicReference,
    profile: &GeoProfile,
    budget: &mut purrdf_geo_kernel::PreparationBudget,
    invocation: &Invocation,
) -> Result<Vec<PreparedCurve>, GeoCallError> {
    let mut curves = Vec::with_capacity(sources.len());
    for source in sources {
        let mut prepared = Vec::with_capacity(source.len());
        for edge in source {
            let edge = match edge {
                EdgeInput::SourceLinear(a, b) => {
                    PreparedEdge::SourceLinear(Box::new(SourceLinearEdge::new(
                        prepare_coordinate(a, reference, budget, invocation)?,
                        prepare_coordinate(b, reference, budget, invocation)?,
                    )?))
                }
                EdgeInput::AzimuthLength(start, azimuth, length) => {
                    let start = prepare_coordinate(start, reference, budget, invocation)?;
                    let mut context = invocation
                        .context(MetricContext::new(reference.clone(), budget.remaining()?)?);
                    let arc = AzimuthLengthArc::from_source_in_context(
                        start,
                        azimuth,
                        length,
                        &mut context,
                    )?;
                    budget.retain(context.work_items(), 0)?;
                    PreparedEdge::AzimuthLength(Box::new(arc))
                }
                EdgeInput::ShortestGeodesic(a, b) => {
                    let start = prepare_coordinate(a, reference, budget, invocation)?;
                    let end = prepare_coordinate(b, reference, budget, invocation)?;
                    let mut context = invocation
                        .context(MetricContext::new(reference.clone(), budget.remaining()?)?);
                    let arc = ShortestGeodesicArc::new(start, end, &mut context)?;
                    budget.retain(context.work_items(), 0)?;
                    PreparedEdge::ShortestGeodesic(Box::new(arc))
                }
                EdgeInput::Image {
                    operation,
                    start,
                    end,
                    epoch,
                } => {
                    let (work, storage) = request::reference_allowance(operation)?;
                    budget.retain(work, storage)?;
                    let mut context = invocation
                        .context(MetricContext::new(reference.clone(), budget.remaining()?)?);
                    let edge = profile.prepare_transformed_edge_named(
                        operation,
                        start,
                        end,
                        epoch.as_ref(),
                        &mut context,
                    )?;
                    budget.retain(context.work_items(), 0)?;
                    edge
                }
            };
            prepared.push(edge);
        }
        curves.push(PreparedCurve::new(prepared));
    }
    Ok(curves)
}

fn prepare_coordinate(
    source: &Coord,
    reference: &purrdf_geo_kernel::GeographicReference,
    budget: &mut purrdf_geo_kernel::PreparationBudget,
    invocation: &Invocation,
) -> Result<PreparedCoordinate, GeoCallError> {
    let mut context =
        invocation.context(MetricContext::new(reference.clone(), budget.remaining()?)?);
    let point = PreparedCoordinate::from_source_in_context(source, reference, &mut context)?;
    budget.retain(context.work_items(), 0)?;
    Ok(point)
}

fn coordinate(value: &Value) -> Result<Coord, GeoCallError> {
    let mut fields = Record::new(value, "original source coordinate")?;
    let coordinate = Coord::new(
        fields.required_with("x", decimal)?,
        fields.required_with("y", decimal)?,
        fields.optional_with("z", decimal)?,
        fields.optional_with("m", decimal)?,
    );
    fields.deny_unknown()?;
    Ok(coordinate)
}

fn edge(value: &Value) -> Result<EdgeInput, GeoCallError> {
    let mut fields = Record::new(value, "explicit curve edge law")?;
    let edge = match fields.tag(
        "law",
        &[
            "source-linear",
            "azimuth-length",
            "shortest-geodesic",
            "operation-image",
        ],
    )? {
        "source-linear" => EdgeInput::SourceLinear(
            fields.required_with("start", coordinate)?,
            fields.required_with("end", coordinate)?,
        ),
        "shortest-geodesic" => EdgeInput::ShortestGeodesic(
            fields.required_with("start", coordinate)?,
            fields.required_with("end", coordinate)?,
        ),
        "operation-image" => EdgeInput::Image {
            operation: Crs::new(fields.required::<String>("operation")?)?,
            start: fields.required_with("start", coordinate)?,
            end: fields.required_with("end", coordinate)?,
            epoch: fields.optional_with("epoch_decimal_year", decimal)?,
        },
        _ => EdgeInput::AzimuthLength(
            fields.required_with("start", coordinate)?,
            fields.required_with("azimuth_degrees", decimal)?,
            Metres::new(fields.required_with("length_metres", decimal)?),
        ),
    };
    fields.deny_unknown()?;
    Ok(edge)
}

fn region(value: &Value) -> Result<RegionInput, GeoCallError> {
    let mut fields = Record::new(value, "explicit surface region")?;
    let region = match fields.tag(
        "kind",
        &["empty", "whole", "geometry", "complement", "native"],
    )? {
        "empty" => RegionInput::Empty,
        "whole" => RegionInput::Whole,
        "geometry" => RegionInput::Carrier(fields.required_with("geometry", request::term)?),
        "native" => RegionInput::Native {
            polygons: fields.required_with("polygons", |value| items_with(value, polygon))?,
            complement: fields.optional("complement")?.unwrap_or(false),
        },
        _ => RegionInput::Complement(fields.required_with("geometry", request::term)?),
    };
    fields.deny_unknown()?;
    Ok(region)
}

fn polygon(value: &Value) -> Result<PolygonInput, GeoCallError> {
    let mut fields = Record::new(value, "native original-law polygon")?;
    let side = match fields.tag("side", &["left", "right"])? {
        "left" => OrientedInterior::Left,
        _ => OrientedInterior::Right,
    };
    let interior = if value.get("interior").is_some() {
        match fields.tag("interior", &["written", "complement"])? {
            "written" => RegionInterior::Written,
            _ => RegionInterior::Complement,
        }
    } else {
        RegionInterior::Written
    };
    let rings = fields.required_with("rings", |value| {
        items_with(value, |value| items_with(value, edge))
    })?;
    fields.deny_unknown()?;
    Ok(PolygonInput {
        rings,
        side,
        interior,
    })
}
