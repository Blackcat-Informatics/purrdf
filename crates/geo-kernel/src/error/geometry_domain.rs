// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native geometry member and extent refusals.

/// The original proof that required an exact source-contact parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceContactReason {
    /// A contact in the written geographic coordinate chart.
    Written,
    /// A contact of the certified reflected source with its comparison source.
    Reflected,
    /// A contact of original sources through the same operation mapping.
    Original,
}
impl purrdf_hash::error::StructuredError for SourceContactReason {
    purrdf_hash::error_visit! {
        Written => "source-contact-written" {},
        Reflected => "source-contact-reflected" {},
        Original => "source-contact-original" {},
    }
}

/// Original start, end and contact positions, in their actual coordinate space.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SourceContactCoordinates {
    /// Written geographic positions without source-axis relabeling.
    Geographic(Box<[crate::LonLat; 3]>),
    /// Original planar positions, including their Z/M ordinates.
    Planar(Box<[crate::Coord; 3]>),
}
impl purrdf_hash::error::StructuredError for SourceContactCoordinates {
    purrdf_hash::error_visit! {
        Geographic(values) => "source-contact-geographic" {
            "start" => Record(&values[0]), "end" => Record(&values[1]), "point" => Record(&values[2])
        },
        Planar(values) => "source-contact-planar" {
            "start" => Record(&values[0]), "end" => Record(&values[1]), "point" => Record(&values[2])
        },
    }
}

/// The original computation that required a selected geodesic edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeodesicEdgeConsumer {
    /// Public signed area integration over the selected geodesic.
    AreaIntegral,
    /// Native selected-arc chart preparation.
    SelectedArc,
}
impl purrdf_hash::error::StructuredError for GeodesicEdgeConsumer {
    purrdf_hash::error_visit! {
        AreaIntegral => "geodesic-edge-area-integral" {},
        SelectedArc => "geodesic-edge-selected-arc" {},
    }
}

/// The original native edge preparation requirement that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreparedEdgeRequirement {
    /// The consuming computation requires a selected geodesic view.
    Geodesic {
        /// Original consuming computation.
        consumer: GeodesicEdgeConsumer,
    },
    /// A transformed source has no admitted original image owner.
    ImageOwner,
}
impl purrdf_hash::error::StructuredError for PreparedEdgeRequirement {
    purrdf_hash::error_visit! {
        Geodesic { consumer } => "prepared-edge-requirement-geodesic" { "consumer" => Record(consumer) },
        ImageOwner => "prepared-edge-requirement-image-owner" {},
    }
}

/// Why an original written fragment has no open chart tangent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WrittenTangentReason {
    /// Its interior point is a physical pole.
    PhysicalPole,
    /// Both actual source-coordinate differences are zero.
    Zero,
}
impl purrdf_hash::error::StructuredError for WrittenTangentReason {
    purrdf_hash::error_visit! {
        PhysicalPole => "written-tangent-physical-pole" {},
        Zero => "written-tangent-zero" {},
    }
}

/// Where the original distance search established missing nonempty support.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DistanceSupportStage {
    /// An originally empty region has no primitive support.
    OriginalEmpty,
    /// Membership did not witness an intersection and one boundary is empty.
    NoBoundary,
    /// No complete candidate supplied an upper distance bound.
    NoCandidate,
}
impl purrdf_hash::error::StructuredError for DistanceSupportStage {
    purrdf_hash::error_visit! {
        OriginalEmpty => "distance-support-original-empty" {},
        NoBoundary => "distance-support-no-boundary" {},
        NoCandidate => "distance-support-no-candidate" {},
    }
}

/// The original edge computation with an undefined exact quotient.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EdgeQuotientOperation {
    /// Interpolation of an ordinate inside an open slab.
    SlabOrdinate,
    /// A buffer edge's intersection with the longitude cut.
    CrossedMeridian,
}
impl purrdf_hash::error::StructuredError for EdgeQuotientOperation {
    purrdf_hash::error_visit! {
        SlabOrdinate => "edge-quotient-slab-ordinate" {},
        CrossedMeridian => "edge-quotient-crossed-meridian" {},
    }
}

/// The original membership law whose complete input count was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MembershipInput {
    /// The per-ring classes of one polygon.
    PolygonRings,
    /// A constant region, whose polygon-class list must be empty.
    ConstantRegion {
        /// Whether the original constant region is the whole surface.
        whole: bool,
    },
    /// The per-polygon classes of a union or its closed complement.
    RegionPolygons {
        /// Whether the original union is complemented.
        complementary: bool,
    },
}
impl purrdf_hash::error::StructuredError for MembershipInput {
    purrdf_hash::error_visit! {
        PolygonRings => "membership-polygon-rings" {},
        ConstantRegion { whole } => "membership-constant-region" { "whole" => Boolean(*whole) },
        RegionPolygons { complementary } => "membership-region-polygons" { "complementary" => Boolean(*complementary) },
    }
}

/// Original operands for a geometry-domain refusal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GeometryDomainError {
    /// A still-unresolved oriented integral has no retained refinement panel.
    EmptyOrientedIntegral {
        /// Original selected line count.
        line_count: usize,
        /// Original numerical iteration bound.
        iterations: u32,
        /// Original panel depth bound.
        max_depth: u32,
        /// Original query normal, requested parameter domain and current total.
        enclosures: Box<[purrdf_xsd::math::FixedInterval; 5]>,
    },
    /// The original exact profile value has no terminating decimal representation.
    NonDecimalParameter {
        /// Original exact parameter.
        value: crate::context::Retained<crate::Rat>,
        /// Original caller-selected rounding law, without implying a finite scale.
        rounding: purrdf_xsd::ieee::ratio::Rounding,
    },
    /// A complete native contact prevents the original ring from being Jordan.
    RingContact {
        /// True for a non-endpoint contact form, false for an unselected endpoint.
        non_jordan: bool,
        /// Original first edge index.
        first_edge: usize,
        /// Original second edge index.
        second_edge: usize,
        /// Original ring edge count.
        edge_count: usize,
        /// Original first refused contact, with its full native witness.
        contact: Box<crate::atlas::arcs::CurveIntersection>,
    },
    /// Two nonadjacent native proof panels of one edge have a certified contact.
    EdgeSelfContact {
        /// Original first panel index.
        first_panel: usize,
        /// Original second panel index.
        second_panel: usize,
        /// Original panel count at the failure.
        panel_count: usize,
        /// Original unrounded parameter domains used by the contact search.
        domain: Box<[purrdf_xsd::math::FixedInterval; 2]>,
        /// Original first certified contact.
        contact: Box<crate::atlas::arcs::CurveIntersection>,
    },
    /// A proved contact has no exact parameter on its original source edge.
    SourceContactParameter {
        /// Original proof/coordinate-law context.
        reason: SourceContactReason,
        /// Whether the right comparison edge failed; left is checked first.
        right: bool,
        /// Complete original endpoints and actual contact point.
        coordinates: SourceContactCoordinates,
    },
    /// The original edge lacks a required native preparation or image owner.
    EdgeRequirement {
        /// Original preparation requirement.
        requirement: PreparedEdgeRequirement,
        /// Original interpolation law.
        kind: crate::PreparedEdgeKind,
        /// Original edge binding.
        binding: crate::GeoBindingId,
        /// Original source coordinates in endpoint order, including Z/M.
        source: Vec<crate::Coord>,
    },
    /// The original polygon has no source chart for written-boundary classification.
    MissingWrittenChart {
        /// Original polygon position.
        index: usize,
        /// Original complete polygon count.
        polygon_count: usize,
        /// Original ring count.
        ring_count: usize,
        /// Original source binding.
        binding: crate::GeoBindingId,
        /// Original explicitly selected traversal interior, if any.
        oriented: Option<crate::OrientedInterior>,
        /// Original closed-support declaration.
        closed_support: bool,
        /// Original selected region interior.
        interior: crate::RegionInterior,
    },
    /// An original written fragment has no open tangent in its source chart.
    WrittenTangent {
        /// Original first failed tangent requirement.
        reason: WrittenTangentReason,
        /// Original classification point, fragment start and fragment end.
        points: Box<[crate::Coord; 3]>,
        /// Actual coordinate differences, absent when the earlier pole check failed.
        difference: Option<[crate::context::Retained<crate::Rat>; 2]>,
    },
    /// A ring has no edge from which to select a Jordan interior.
    EmptyOrientedRing {
        /// Original edge count.
        edges: usize,
    },
    /// An original written or shortest-geodesic ring edge is constant.
    ConstantRingEdge {
        /// Original edge position in traversal order.
        index: usize,
        /// Original complete ring edge count.
        edge_count: usize,
        /// Original interpolation law.
        kind: crate::PreparedEdgeKind,
        /// Original source, normalized position and binding at both endpoints.
        endpoints: Box<[crate::PreparedCoordinate; 2]>,
    },
    /// A point-only materialization was given a general prepared source.
    PointMaterialization {
        /// Original source identity and preparation admission.
        source: crate::PreparedSourceReceipt,
        /// Original explicit point count.
        points: usize,
        /// Original symbolic point count.
        symbolic_points: usize,
        /// Original curve count.
        curves: usize,
        /// Original selected region form.
        region: crate::PreparedRegionKind,
    },
    /// The original distance search cannot find two nonempty supports.
    DistanceSupport {
        /// Original failed search stage.
        stage: DistanceSupportStage,
        /// Original left and right selected region forms.
        regions: [crate::PreparedRegionKind; 2],
        /// Original left and right primitive counts.
        primitives: [usize; 2],
    },
    /// The original planar arguments have no pair of nonempty point sets.
    PlanarDistanceSupport {
        /// Original left and right geometry kinds.
        kinds: [crate::GeometryKind; 2],
        /// Original left and right empty-set membership.
        empty: [bool; 2],
    },
    /// A membership law received an incomplete or unexpected class list.
    Membership {
        /// Original membership law.
        input: MembershipInput,
        /// Original required class count.
        expected: usize,
        /// Original supplied classes, in order and without deduplication.
        actual: Vec<bool>,
    },
    /// An original edge computation has no exact parameter quotient.
    EdgeQuotient {
        /// Original consuming computation.
        operation: EdgeQuotientOperation,
        /// Original endpoints in source order, including any extra ordinates.
        endpoints: Box<[crate::Coord; 2]>,
        /// Original requested slab coordinate.
        x: crate::context::Retained<crate::Rat>,
        /// Actual numerator of the refused parameter quotient.
        offset: crate::context::Retained<crate::Rat>,
        /// Actual denominator of the refused parameter quotient.
        width: crate::context::Retained<crate::Rat>,
    },
    /// The original closed box has reversed latitude bounds.
    ReversedBox {
        /// Original west, south, east and north bounds, in declaration order.
        bounds: [crate::context::Retained<crate::Rat>; 4],
    },
    /// The original certified angular enclosure is empty after latitude clipping.
    EmptyEnclosure {
        /// Original unrounded longitude and latitude intervals, in axis order.
        intervals: Box<[purrdf_xsd::math::FixedInterval; 2]>,
        /// Actual compared clipped lower latitude.
        south: crate::context::Retained<crate::Rat>,
        /// Actual compared clipped upper latitude.
        north: crate::context::Retained<crate::Rat>,
        /// First failed axis, longitude before latitude.
        axis: usize,
    },
    /// A native polygon has no ring from which to select an interior.
    EmptyOrientedPolygon {
        /// Original selected traversal interior, if supplied.
        oriented: Option<crate::OrientedInterior>,
        /// Original closed-support declaration.
        closed_support: bool,
    },
    /// The original geometry literals name different coordinate systems.
    CrsMismatch {
        /// Original left reference identifier.
        first: String,
        /// Original right reference identifier.
        second: String,
    },
    /// The requested bound has no coordinate in the original geometry.
    MissingBound {
        /// Original scalar function local name.
        function: &'static str,
        /// Original requested ordinate.
        ordinate: crate::measure::Ordinate,
        /// Whether the original geometry contains positions.
        has_positions: bool,
    },
    /// A one-based member index is outside the original member count.
    MissingMember {
        /// Original requested index, including a negative value.
        index: i64,
        /// Original member count.
        count: usize,
    },
}

impl purrdf_hash::error::StructuredError for GeometryDomainError {
    purrdf_hash::error_visit! {
        EmptyOrientedIntegral { line_count, iterations, max_depth, enclosures } => "geometry-empty-oriented-integral" {
            "line_count" => Unsigned(*line_count as u128), "iterations" => Unsigned(u128::from(*iterations)), "max_depth" => Unsigned(u128::from(*max_depth)),
            "query" => List(enclosures[..3].iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "parameter" => Record(&enclosures[3]), "total" => Record(&enclosures[4]), "panel_count" => Unsigned(0)
        },
        NonDecimalParameter { value, rounding } => "geometry-non-decimal-parameter" { "value" => Record(&**value), "rounding" => Record(rounding) },
        RingContact { non_jordan, first_edge, second_edge, edge_count, contact } => "geometry-ring-contact" {
            "non_jordan" => Boolean(*non_jordan), "first_edge" => Unsigned(*first_edge as u128),
            "second_edge" => Unsigned(*second_edge as u128), "edge_count" => Unsigned(*edge_count as u128), "contact" => Record(&**contact)
        },
        EdgeSelfContact { first_panel, second_panel, panel_count, domain, contact } => "geometry-edge-self-contact" {
            "first_panel" => Unsigned(*first_panel as u128), "second_panel" => Unsigned(*second_panel as u128), "panel_count" => Unsigned(*panel_count as u128),
            "domain" => List(domain.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))), "contact" => Record(&**contact)
        },
        SourceContactParameter { reason, right, coordinates } => "geometry-source-contact-parameter" {
            "reason" => Record(reason), "right" => Boolean(*right), "coordinates" => Record(coordinates)
        },
        EdgeRequirement { requirement, kind, binding, source } => "geometry-edge-requirement" {
            "requirement" => Record(requirement), "kind" => Record(kind), "binding" => Record(binding),
            "source" => List(source.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value)))
        },
        MissingWrittenChart { index, polygon_count, ring_count, binding, oriented, closed_support, interior } => "geometry-missing-written-chart" {
            "index" => Unsigned(*index as u128), "polygon_count" => Unsigned(*polygon_count as u128), "ring_count" => Unsigned(*ring_count as u128),
            "binding" => Record(binding), "oriented" => Value(match oriented {
                Some(value) => purrdf_hash::error::ErrorValue::Record(value), None => purrdf_hash::error::ErrorValue::Absent,
            }), "closed_support" => Boolean(*closed_support), "interior" => Record(interior)
        },
        WrittenTangent { reason, points, difference } => "geometry-written-tangent" {
            "reason" => Record(reason), "points" => List(points.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "dx" => Value(match difference { Some(value) => purrdf_hash::error::ErrorValue::Record(&*value[0]), None => purrdf_hash::error::ErrorValue::Absent }),
            "dy" => Value(match difference { Some(value) => purrdf_hash::error::ErrorValue::Record(&*value[1]), None => purrdf_hash::error::ErrorValue::Absent })
        },
        EmptyOrientedRing { edges } => "geometry-empty-oriented-ring" { "edges" => Unsigned(*edges as u128) },
        ConstantRingEdge { index, edge_count, kind, endpoints } => "geometry-constant-ring-edge" {
            "index" => Unsigned(*index as u128), "edge_count" => Unsigned(*edge_count as u128), "kind" => Record(kind),
            "endpoints" => List(endpoints.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value)))
        },
        PointMaterialization { source, points, symbolic_points, curves, region } => "geometry-point-materialization" {
            "source" => Record(source), "points" => Unsigned(*points as u128),
            "symbolic_points" => Unsigned(*symbolic_points as u128), "curves" => Unsigned(*curves as u128), "region" => Record(region)
        },
        DistanceSupport { stage, regions, primitives } => "geometry-distance-support" {
            "stage" => Record(stage), "regions" => List(regions.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "primitives" => List(primitives.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(*value as u128)))
        },
        PlanarDistanceSupport { kinds, empty } => "geometry-planar-distance-support" {
            "kinds" => List(kinds.iter().map(|value| purrdf_hash::error::ErrorValue::Code(value.wkt_keyword()))),
            "empty" => List(empty.iter().map(|value| purrdf_hash::error::ErrorValue::Boolean(*value)))
        },
        Membership { input, expected, actual } => "geometry-membership-input" {
            "input" => Record(input), "expected" => Unsigned(*expected as u128),
            "actual" => List(actual.iter().map(|value| purrdf_hash::error::ErrorValue::Boolean(*value)))
        },
        EdgeQuotient { operation, endpoints, x, offset, width } => "geometry-edge-quotient" {
            "operation" => Record(operation),
            "first" => Record(&endpoints[0]), "second" => Record(&endpoints[1]), "x" => Record(&**x),
            "offset" => Record(&**offset), "width" => Record(&**width)
        },
        ReversedBox { bounds } => "geometry-reversed-box" {
            "bounds" => List(bounds.iter().map(|value| purrdf_hash::error::ErrorValue::Record(&**value)))
        },
        EmptyEnclosure { intervals, south, north, axis } => "geometry-empty-enclosure" {
            "longitude" => Record(&intervals[0]), "latitude" => Record(&intervals[1]),
            "south" => Record(&**south), "north" => Record(&**north), "axis" => Unsigned(*axis as u128)
        },
        EmptyOrientedPolygon { oriented, closed_support } => "geometry-empty-oriented-polygon" {
            "oriented" => Value(match oriented {
                Some(value) => purrdf_hash::error::ErrorValue::Record(value),
                None => purrdf_hash::error::ErrorValue::Absent,
            }),
            "closed_support" => Boolean(*closed_support), "ring_count" => Unsigned(0)
        },
        CrsMismatch { first, second } => "geometry-crs-mismatch" { "first" => Iri(first), "second" => Iri(second) },
        MissingBound { function, ordinate, has_positions } => "geometry-missing-bound" {
            "function" => Code(function), "ordinate" => Record(ordinate), "has_positions" => Boolean(*has_positions)
        },
        MissingMember { index, count } => "geometry-missing-member" {
            "index" => Signed(i128::from(*index)), "count" => Unsigned(*count as u128)
        },
    }
}

impl GeometryDomainError {
    pub(crate) fn empty_oriented_integral(
        query: &[purrdf_xsd::math::FixedInterval; 3],
        parameter: &purrdf_xsd::math::FixedInterval,
        total: &purrdf_xsd::math::FixedInterval,
        line_count: usize,
        iterations: u32,
        max_depth: u32,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let [x, y, z] =
                crate::context::retained::produce_persistent_numeric_from(query, progress)?
                    .into_value();
            let parameter =
                crate::context::retained::produce_persistent_numeric_from(parameter, progress)?
                    .into_value();
            let total = crate::context::retained::produce_persistent_numeric_from(total, progress)?
                .into_value();
            crate::context::Retained::produce(
                progress,
                |memory| {
                    let enclosures = memory
                        .boxed([x, y, z, parameter, total])
                        .map_err(super::GeoError::Storage)?;
                    Ok(Self::EmptyOrientedIntegral {
                        line_count,
                        iterations,
                        max_depth,
                        enclosures,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn ring_contact(
        contact: &crate::atlas::arcs::CurveIntersection,
        position: [usize; 3],
        non_jordan: bool,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = crate::context::retained::produce_persistent_numeric_from(contact, progress)
            .and_then(|contact| {
                crate::context::Retained::produce(
                    progress,
                    |memory| {
                        let contact = memory
                            .boxed(contact.into_value())
                            .map_err(super::GeoError::Storage)?;
                        let [first_edge, second_edge, edge_count] = position;
                        Ok(Self::RingContact {
                            non_jordan,
                            first_edge,
                            second_edge,
                            edge_count,
                            contact,
                        })
                    },
                    |_, _| Ok(()),
                )
            });
        captured.map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn edge_self_contact(
        contact: &crate::atlas::arcs::CurveIntersection,
        domain: &[purrdf_xsd::math::FixedInterval; 2],
        position: [usize; 3],
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let contact =
                crate::context::retained::produce_persistent_numeric_from(contact, progress)?
                    .into_value();
            let domain =
                crate::context::retained::produce_persistent_numeric_from(domain, progress)?
                    .into_value();
            crate::context::Retained::produce(
                progress,
                |memory| {
                    let contact = memory.boxed(contact).map_err(super::GeoError::Storage)?;
                    let domain = memory.boxed(domain).map_err(super::GeoError::Storage)?;
                    let [first_panel, second_panel, panel_count] = position;
                    Ok(Self::EdgeSelfContact {
                        first_panel,
                        second_panel,
                        panel_count,
                        domain,
                        contact,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn contact_parameter<T: crate::context::retained::NativeDetached>(
        points: [&T; 3],
        reason: SourceContactReason,
        right: bool,
        wrap: fn(Box<[T; 3]>) -> SourceContactCoordinates,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let start =
                crate::context::retained::produce_persistent_numeric_from(points[0], progress)?
                    .into_value();
            let end =
                crate::context::retained::produce_persistent_numeric_from(points[1], progress)?
                    .into_value();
            let point =
                crate::context::retained::produce_persistent_numeric_from(points[2], progress)?
                    .into_value();
            crate::context::Retained::produce(
                progress,
                |memory| {
                    let coordinates = wrap(
                        memory
                            .boxed([start, end, point])
                            .map_err(super::GeoError::Storage)?,
                    );
                    Ok(Self::SourceContactParameter {
                        reason,
                        right,
                        coordinates,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn edge_requirement(
        edge: &crate::PreparedEdge,
        requirement: PreparedEdgeRequirement,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        crate::context::Retained::produce(
            progress,
            |memory| {
                let source = memory
                    .try_map(edge.original_coordinates(), |value, memory| {
                        value.detached_with_memory(memory)
                    })
                    .map_err(super::GeoError::Storage)?;
                Ok(Self::EdgeRequirement {
                    requirement,
                    kind: edge.kind(),
                    binding: edge.binding_id(),
                    source,
                })
            },
            |value, original| {
                let Self::EdgeRequirement { source, .. } = value else {
                    unreachable!("original edge requirement factory")
                };
                for point in source {
                    point
                        .publish_detached(original)
                        .map_err(super::GeoError::Storage)?;
                }
                Ok(())
            },
        )
        .map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn written_tangent(
        points: [&crate::Coord; 3],
        difference: Option<[&crate::Rat; 2]>,
        reason: WrittenTangentReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let point =
                crate::context::retained::produce_persistent_numeric_from(points[0], progress)?
                    .into_value();
            let from =
                crate::context::retained::produce_persistent_numeric_from(points[1], progress)?
                    .into_value();
            let to =
                crate::context::retained::produce_persistent_numeric_from(points[2], progress)?
                    .into_value();
            let difference = difference
                .map(|value| {
                    Ok::<_, super::GeoError>([
                        super::GeoError::diagnostic_number(value[0], progress)?,
                        super::GeoError::diagnostic_number(value[1], progress)?,
                    ])
                })
                .transpose()?;
            crate::context::Retained::produce(
                progress,
                |memory| {
                    let points = memory
                        .boxed([point, from, to])
                        .map_err(super::GeoError::Storage)?;
                    Ok(Self::WrittenTangent {
                        reason,
                        points,
                        difference,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn constant_ring_edge(
        endpoints: [&crate::PreparedCoordinate; 2],
        kind: crate::PreparedEdgeKind,
        index: usize,
        edge_count: usize,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let first =
                crate::context::retained::produce_persistent_numeric_from(endpoints[0], progress)?
                    .into_value();
            let second =
                crate::context::retained::produce_persistent_numeric_from(endpoints[1], progress)?
                    .into_value();
            crate::context::Retained::produce(
                progress,
                |memory| {
                    let endpoints = memory
                        .boxed([first, second])
                        .map_err(super::GeoError::Storage)?;
                    Ok(Self::ConstantRingEdge {
                        index,
                        edge_count,
                        kind,
                        endpoints,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    /// Preserve the original planar support refusal through native admission.
    #[must_use]
    pub fn planar_distance_support(
        first: &crate::Geometry,
        second: &crate::Geometry,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> super::GeoError {
        let mut progress = crate::context::WorkProgress::integer(observer);
        let result = crate::context::Retained::produce(
            &mut progress,
            |memory| {
                let empty = [
                    first
                        .is_empty_with_memory(memory)
                        .map_err(super::GeoError::Storage)?,
                    second
                        .is_empty_with_memory(memory)
                        .map_err(super::GeoError::Storage)?,
                ];
                Ok(Self::PlanarDistanceSupport {
                    kinds: [first.kind(), second.kind()],
                    empty,
                })
            },
            |_, _| Ok(()),
        );
        match result {
            Ok(cause) => super::GeoError::GeometryDomain(cause),
            Err(error) => error,
        }
    }

    pub(crate) fn distance_support(
        stage: DistanceSupportStage,
        regions: [&crate::PreparedRegion; 2],
        primitives: [usize; 2],
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::retained_failure_with_progress(
            Self::DistanceSupport {
                stage,
                regions: regions.map(crate::PreparedRegion::kind),
                primitives,
            },
            super::GeoError::GeometryDomain,
            progress,
        )
    }

    pub(crate) fn membership(
        input: MembershipInput,
        expected: usize,
        actual: &[bool],
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        crate::context::Retained::produce(
            progress,
            |memory| {
                let actual = memory
                    .collect(actual.iter().copied())
                    .map_err(super::GeoError::Storage)?;
                Ok(Self::Membership {
                    input,
                    expected,
                    actual,
                })
            },
            |_, _| Ok(()),
        )
        .map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn edge_quotient(
        first: &crate::Coord,
        second: &crate::Coord,
        x: &crate::Rat,
        offset: &crate::Rat,
        width: &crate::Rat,
        operation: EdgeQuotientOperation,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let first = crate::context::retained::produce_persistent_numeric_from(first, progress)?
                .into_value();
            let second =
                crate::context::retained::produce_persistent_numeric_from(second, progress)?
                    .into_value();
            let x = super::GeoError::diagnostic_number(x, progress)?;
            let offset = super::GeoError::diagnostic_number(offset, progress)?;
            let width = super::GeoError::diagnostic_number(width, progress)?;
            crate::context::Retained::produce(
                progress,
                |memory| {
                    let endpoints = memory
                        .boxed([first, second])
                        .map_err(super::GeoError::Storage)?;
                    Ok(Self::EdgeQuotient {
                        operation,
                        endpoints,
                        x,
                        offset,
                        width,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn reversed_box(
        bounds: [crate::context::Retained<crate::Rat>; 4],
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        super::GeoError::retained_failure_with_progress(
            Self::ReversedBox { bounds },
            super::GeoError::GeometryDomain,
            progress,
        )
    }

    pub(crate) fn empty_enclosure(
        longitude: &purrdf_xsd::math::FixedInterval,
        latitude: &purrdf_xsd::math::FixedInterval,
        clipped: [&crate::Rat; 2],
        axis: usize,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            let longitude =
                crate::context::retained::produce_persistent_numeric_from(longitude, progress)?
                    .into_value();
            let latitude =
                crate::context::retained::produce_persistent_numeric_from(latitude, progress)?
                    .into_value();
            let south = super::GeoError::diagnostic_number(clipped[0], progress)?;
            let north = super::GeoError::diagnostic_number(clipped[1], progress)?;
            crate::context::Retained::produce(
                progress,
                |memory| {
                    let intervals = memory
                        .boxed([longitude, latitude])
                        .map_err(super::GeoError::Storage)?;
                    Ok(Self::EmptyEnclosure {
                        intervals,
                        south,
                        north,
                        axis,
                    })
                },
                |_, _| Ok(()),
            )
        })();
        captured.map_or_else(|error| error, super::GeoError::GeometryDomain)
    }

    pub(crate) fn crs_pair(
        first: &crate::Crs,
        second: &crate::Crs,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        crate::context::Retained::produce(
            progress,
            |memory| {
                memory.try_scope(|memory| {
                    Ok(Self::CrsMismatch {
                        first: memory
                            .string(first.as_str())
                            .map_err(super::GeoError::Storage)?,
                        second: memory
                            .string(second.as_str())
                            .map_err(super::GeoError::Storage)?,
                    })
                })
            },
            |_, _| Ok(()),
        )
        .map_or_else(|error| error, super::GeoError::GeometryDomain)
    }
}

impl core::fmt::Display for GeometryDomainError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyOrientedIntegral { .. } => f.write_str("empty oriented integral"),
            Self::NonDecimalParameter { .. } => {
                f.write_str("custom profile parameter has no exact decimal representation")
            }
            Self::RingContact { non_jordan, .. } => f.write_str(if *non_jordan {
                "non-Jordan oriented ring contact"
            } else {
                "self contact in oriented ring"
            }),
            Self::EdgeSelfContact { .. } => f.write_str("self contact on an oriented edge"),
            Self::SourceContactParameter { reason, .. } => f.write_str(match reason {
                SourceContactReason::Written => "proved source-linear contact parameter",
                SourceContactReason::Reflected => "proved reflected source contact parameter",
                SourceContactReason::Original => "proved original source contact parameter",
            }),
            Self::EdgeRequirement { requirement, .. } => f.write_str(match requirement {
                PreparedEdgeRequirement::Geodesic {
                    consumer: GeodesicEdgeConsumer::AreaIntegral,
                } => "selected geodesic area requires a geodesic edge",
                PreparedEdgeRequirement::Geodesic {
                    consumer: GeodesicEdgeConsumer::SelectedArc,
                } => "expected selected geodesic edge",
                PreparedEdgeRequirement::ImageOwner => "admitted original image owner",
            }),
            Self::MissingWrittenChart { .. } => {
                f.write_str("written boundary sides require source charts")
            }
            Self::WrittenTangent { reason, .. } => f.write_str(match reason {
                WrittenTangentReason::PhysicalPole => "a physical pole has no open chart tangent",
                WrittenTangentReason::Zero => "zero written noded tangent",
            }),
            Self::EmptyOrientedRing { .. } => f.write_str("empty oriented ring"),
            Self::ConstantRingEdge { .. } => f.write_str("constant edge in oriented ring"),
            Self::PointMaterialization { .. } => {
                f.write_str("point materialization requires a complete point collection")
            }
            Self::DistanceSupport { .. } => {
                f.write_str("distance requires two nonempty geometries")
            }
            Self::PlanarDistanceSupport { .. } => f.write_str("the distance to an empty geometry is undefined: there is no point of the empty set for the distance to be measured to, and 0 would say the two geometries touch"),
            Self::Membership { input, .. } => f.write_str(match input {
                MembershipInput::PolygonRings => "polygon membership ring count mismatch",
                MembershipInput::ConstantRegion { .. } => {
                    "constant region membership count mismatch"
                }
                MembershipInput::RegionPolygons { .. } => {
                    "region membership polygon count mismatch"
                }
            }),
            Self::EdgeQuotient { operation, .. } => f.write_str(match operation {
                EdgeQuotientOperation::SlabOrdinate => "vertical edge has no open-slab ordinate",
                EdgeQuotientOperation::CrossedMeridian => {
                    "crossed meridian has zero longitude variation"
                }
            }),
            Self::ReversedBox { .. } => f.write_str("box south exceeds north"),
            Self::EmptyEnclosure { .. } => f.write_str("empty validated geographic enclosure"),
            Self::EmptyOrientedPolygon { .. } => f.write_str("empty native oriented polygon"),
            Self::CrsMismatch { first, second } => write!(
                f,
                "the two geometries are in different coordinate reference systems (<{first}> and <{second}>); purrdf-geo reprojects nothing, so it refuses rather than treating their ordinates as comparable"
            ),
            Self::MissingBound {
                function,
                ordinate,
                has_positions,
            } => {
                use crate::measure::Ordinate;
                let reason = match (ordinate, has_positions) {
                    (Ordinate::X | Ordinate::Y, _) => {
                        "an empty geometry: the empty set has no extent, and 0 would be a coordinate the data never contained"
                    }
                    (Ordinate::Z, false) => {
                        "an empty geometry: the empty set has no positions, so it has no elevation, and 0 would be one the data never stated"
                    }
                    (Ordinate::Z, true) => {
                        "a geometry with no Z ordinate: this geometry is planar, so it has no elevation at all, and 0 would be an elevation the data never stated"
                    }
                };
                write!(f, "geof:{function} has no answer for {reason}")
            }
            Self::MissingMember { index, count } => write!(
                f,
                "geof:geometryN was asked for member {index} of a geometry with {count} member(s); the index is 1-based, and outside that range there is no member — answering with an empty geometry would be a different question's answer"
            ),
        }
    }
}
impl std::error::Error for GeometryDomainError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkProgress;
    use crate::{GeoError, Rat};
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::{Capture, Event};

    #[test]
    fn closed_box_refusal_preserves_original_bound_order_and_coordinate_priority() {
        let bounds = [-170, 5, 170, -5].map(Rat::from_i64);
        let error = crate::cells::ClosedBox::new(
            bounds[0].clone(),
            bounds[1].clone(),
            bounds[2].clone(),
            bounds[3].clone(),
        )
        .unwrap_err();
        let GeoError::GeometryDomain(cause) = error.original() else {
            panic!("native box bounds")
        };
        let GeometryDomainError::ReversedBox { bounds: original } = &**cause else {
            panic!("original reversed box")
        };
        for (original, expected) in original.iter().zip(&bounds) {
            assert_eq!(&**original, expected);
        }
        assert_eq!(error.detail(), "box south exceeds north");
        assert_eq!(
            Capture::read(error.original())
                .child("cause")
                .records("bounds")
                .len(),
            4
        );
        let error = crate::cells::ClosedBox::new(
            Rat::from_i64(181),
            Rat::from_i64(5),
            Rat::zero(),
            Rat::from_i64(-5),
        )
        .unwrap_err();
        assert!(matches!(
            error.original(),
            GeoError::CoordinateOutOfRange { .. }
        ));
    }

    #[test]
    fn enclosure_refusal_preserves_unrounded_intervals_and_actual_latitude_clip() {
        use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathLimits};
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let longitude = FixedInterval::from_i64(0, &mut math).unwrap();
        let latitude = FixedInterval::from_i64(91, &mut math).unwrap();
        let region = crate::PreparedRegion::Polygons(Vec::new().into());
        let curve = crate::PreparedCurve::new(Vec::new());
        for ring in [false, true] {
            let mut context = crate::MetricContext::wgs84().unwrap();
            let mut progress = WorkProgress::integer(None);
            let error = if ring {
                crate::atlas::locate_ring_enclosure(
                    &longitude,
                    &latitude,
                    &curve,
                    &mut context,
                    &mut progress,
                )
            } else {
                crate::atlas::locate_enclosure(
                    &longitude,
                    &latitude,
                    &region,
                    &mut context,
                    &mut progress,
                )
            }
            .unwrap_err();
            let GeoError::GeometryDomain(cause) = error.original() else {
                panic!("native angular enclosure")
            };
            let GeometryDomainError::EmptyEnclosure {
                intervals,
                south,
                north,
                axis,
            } = &**cause
            else {
                panic!("original enclosure bounds")
            };
            assert_eq!((&intervals[0], &intervals[1]), (&longitude, &latitude));
            assert_eq!(
                (&**south, &**north, *axis),
                (&Rat::from_i64(91), &Rat::from_i64(90), 1)
            );
            assert_eq!(error.detail(), "empty validated geographic enclosure");
            assert_eq!(context.current_workspace_bytes(), 0);
            drop(context);
            assert_eq!(
                Capture::read(error.original()).child("cause").code,
                "geometry-empty-enclosure"
            );
        }
    }

    #[test]
    fn constant_membership_refusals_keep_complete_ordered_classes() {
        let classes = [true, false, true];
        for (region, whole) in [
            (crate::PreparedRegion::Empty, false),
            (crate::PreparedRegion::Whole, true),
        ] {
            let error = crate::atlas::region_membership(
                &region,
                &classes,
                &mut WorkProgress::integer(None),
            )
            .unwrap_err();
            let GeoError::GeometryDomain(cause) = error.original() else {
                panic!("native membership count")
            };
            assert!(
                matches!(&**cause, GeometryDomainError::Membership { input: MembershipInput::ConstantRegion { whole: original }, expected: 0, actual }
                if *original == whole && actual == &classes)
            );
            let capture = Capture::read(error.original()).child("cause");
            assert!(matches!(capture.field("expected"), ErrorValue::Unsigned(0)));
            assert!(
                matches!(capture.child("input").field("whole"), ErrorValue::Boolean(value) if value == whole)
            );
            assert!(
                capture
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::BeginList(Some("actual"), 3)))
            );
            let actual = capture
                .events
                .iter()
                .filter_map(|event| match event {
                    Event::Value(None, ErrorValue::Boolean(value)) => Some(*value),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, classes);
            assert_eq!(error.detail(), "constant region membership count mismatch");
        }
    }
}
