// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! DE-9IM witnesses on the original-parameter contact graph. No contact's
//! floating or decimal enclosure becomes a replacement geometry vertex.

use super::{
    Address, Cut, Inventory, Overlap, edge, exact_fragment_point, fragment_point, initialize,
    midpoint, node_pairs, polygons, reserve_metadata, rings, root,
};
use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, geo_math_error};
use crate::operation::OperationImageCurve;
use crate::{Dim, IntersectionMatrix, LonLat, PreparedCoordinate, SourceLinearEdge};
use crate::{
    GeoError, MetricContext, PreparedCurve, PreparedEdge, PreparedGeometry, PreparedRegion, Rat,
    Set,
};
use purrdf_core::SmallVec;
use purrdf_xsd::integer::ExactOperation::{Linear, RationalCompare};
use purrdf_xsd::math::FixedInterval;
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Kind {
    Curve,
    Point,
}

struct Layout<'a> {
    inputs: [&'a PreparedRegion; 2],
    polygon_offsets: [usize; 3],
    curves: Vec<(usize, Kind)>,
}

#[derive(Default)]
struct NodeWitness<'a> {
    representative: Option<(Address, &'a Cut)>,
    incident: SmallVec<[Address; 4]>,
    point_member: [bool; 2],
    curve_member: [bool; 2],
    lower_curve: [bool; 2],
    odd: [bool; 2],
    inside: [bool; 2],
    outside: [bool; 2],
    boundary: [bool; 2],
}

#[derive(Clone, Copy)]
struct ClosedQuery<'a> {
    address: Address,
    bounds: [&'a Rat; 2],
    incident: &'a [Address],
}

struct FragmentLocations {
    faces: [[Set; 2]; 2],
    closed: [Set; 2],
}

#[derive(Clone, Copy)]
struct FragmentWitness<'a> {
    address: Address,
    ends: &'a [Cut],
    locations: &'a FragmentLocations,
}

#[derive(Clone, Copy)]
struct PointWitness<'a> {
    address: Address,
    cut: &'a Cut,
    isolated: [bool; 2],
}

/// Consumers observe the same complete physical-stratum proof. The matrix
/// path needs no owned copies; boundary publication retains original cuts.
trait GraphOutput {
    fn fragment(
        &mut self,
        inventory: &Inventory,
        witness: FragmentWitness<'_>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError>;

    fn point(
        &mut self,
        inventory: &Inventory,
        witness: PointWitness<'_>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError>;

    fn face(&mut self, location: [Set; 2]);
}

struct MatrixOnly;

impl GraphOutput for MatrixOnly {
    fn fragment(
        &mut self,
        _: &Inventory,
        _: FragmentWitness<'_>,
        _: &mut MetricContext,
        _: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        Ok(())
    }

    fn point(
        &mut self,
        _: &Inventory,
        _: PointWitness<'_>,
        _: &mut MetricContext,
        _: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        Ok(())
    }

    fn face(&mut self, _: [Set; 2]) {}
}

struct SelectedStrata {
    fragments: Vec<super::NativeBoundaryFragment>,
    points: Vec<super::NativeSelectedPoint>,
    has_areal_faces: bool,
}

impl SelectedStrata {
    fn admit_output(&self, context: &MetricContext) -> Result<(), GeoError> {
        let count = self.fragments.len().checked_add(self.points.len()).ok_or(
            GeoError::ArithmeticOverflow("selected stratum output count"),
        )?;
        if count as u64 >= context.policy().limits().max_output_elements {
            return Err(GeoError::OutputExhausted {
                limit: context.policy().limits().max_output_elements,
            });
        }
        Ok(())
    }
}

fn copy_parameter(
    inventory: &Inventory,
    cut: &Cut,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<super::SourceParameter, GeoError> {
    context.charge_work(super::node_depth_bound(inventory.parents.len()) + 1)?;
    progress.context_poll(context)?;
    Ok(super::SourceParameter {
        source: inventory.source.clone(),
        definition: super::copy_definition(&cut.definition, context, progress)?,
        bounds: super::copy_bounds([&cut.bounds.0, &cut.bounds.1], context, progress)?,
        node: super::physical_node(inventory, cut.node),
    })
}

impl GraphOutput for SelectedStrata {
    fn fragment(
        &mut self,
        inventory: &Inventory,
        witness: FragmentWitness<'_>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        let faces = witness.locations.faces[0];
        let stratum = if faces[0] != faces[1] {
            super::SelectedFragmentStratum::ArealBoundary
        } else if faces == [Set::Exterior; 2] && witness.locations.closed[0] != Set::Exterior {
            super::SelectedFragmentStratum::CurveInterior
        } else {
            return Ok(());
        };
        self.admit_output(context)?;
        let parameters = [
            copy_parameter(inventory, &witness.ends[0], context, progress)?,
            copy_parameter(inventory, &witness.ends[1], context, progress)?,
        ];
        let reversed =
            stratum == super::SelectedFragmentStratum::ArealBoundary && faces[0] == Set::Exterior;
        let id = crate::profile::hash_fields(
            super::FRAGMENT_DOMAIN,
            [
                inventory.source.id().as_bytes().as_slice(),
                &(witness.address.ring as u64).to_be_bytes(),
                &(witness.address.edge as u64).to_be_bytes(),
                parameters[0].node.as_bytes(),
                parameters[1].node.as_bytes(),
                &[u8::from(reversed)],
            ],
        );
        self.fragments.push(super::NativeBoundaryFragment {
            source: inventory.source.clone(),
            address: witness.address,
            parameters,
            reversed,
            stratum,
            id,
        });
        Ok(())
    }

    fn point(
        &mut self,
        inventory: &Inventory,
        witness: PointWitness<'_>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        if witness.isolated[0] {
            self.admit_output(context)?;
            self.points.push(super::NativeSelectedPoint {
                source: inventory.source.clone(),
                address: witness.address,
                parameter: copy_parameter(inventory, witness.cut, context, progress)?,
            });
        }
        Ok(())
    }

    fn face(&mut self, location: [Set; 2]) {
        self.has_areal_faces |= location[0] == Set::Interior;
    }
}

/// Publish every selected physical stratum from the relation witness body.
pub(super) fn selected_strata(
    inventory: &Inventory,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<super::NativeBoundaryArrangement, GeoError> {
    let mut count = 0usize;
    let mut limb_bytes = 0u64;
    for cuts in &inventory.cuts {
        metadata_work(1, context, progress)?;
        count = count
            .checked_add(cuts.len().saturating_sub(1))
            .ok_or(GeoError::ArithmeticOverflow("selected fragment capacity"))?;
        for cut in cuts {
            metadata_work(1, context, progress)?;
            let bytes = (cut.bounds.0.allocated_bytes() as u64)
                .checked_add(cut.bounds.1.allocated_bytes() as u64)
                .and_then(|bytes| {
                    bytes.checked_add(match &cut.definition {
                        super::Definition::Exact(value) => value.allocated_bytes() as u64,
                        super::Definition::Contact { .. } => 0,
                    })
                })
                .ok_or(GeoError::ArithmeticOverflow("selected parameter storage"))?;
            // A cut occurs at most twice as a fragment endpoint and once as
            // an isolated physical node; every live exact copy is covered.
            limb_bytes = bytes
                .checked_mul(3)
                .and_then(|extra| limb_bytes.checked_add(extra))
                .ok_or(GeoError::ArithmeticOverflow("selected parameter storage"))?;
        }
    }
    let storage = (count as u64)
        .checked_mul(size_of::<super::NativeBoundaryFragment>() as u64)
        .and_then(|bytes| {
            (inventory.nodes.len() as u64)
                .checked_mul(size_of::<super::NativeSelectedPoint>() as u64)
                .and_then(|extra| bytes.checked_add(extra))
        })
        .and_then(|bytes| bytes.checked_add(limb_bytes))
        .and_then(|bytes| bytes.checked_add(size_of::<super::NativeBoundaryArrangement>() as u64))
        .ok_or(GeoError::ArithmeticOverflow("selected stratum storage"))?;
    context.admit_workspace(storage)?;
    let result = (|| {
        let mut output = SelectedStrata {
            fragments: Vec::new(),
            points: Vec::new(),
            has_areal_faces: false,
        };
        reserve_metadata(&mut output.fragments, count, context, progress)?;
        reserve_metadata(&mut output.points, inventory.nodes.len(), context, progress)?;
        let empty = PreparedRegion::Empty;
        let polygon_count = polygons(inventory.source.region())?.len();
        let layout = Layout {
            inputs: [inventory.source.region(), &empty],
            polygon_offsets: [0, polygon_count, polygon_count],
            curves: Vec::new(),
        };
        witnesses_with(inventory, &layout, &mut output, context, progress)?;
        let owned = selected_storage(&output, context, progress)?;
        Ok(super::NativeBoundaryArrangement {
            source: inventory.source.clone(),
            fragments: output.fragments,
            isolated_points: output.points,
            has_areal_faces: output.has_areal_faces,
            retained: inventory
                .source
                .retained_workspace_bytes()
                .checked_add(owned)
                .ok_or(GeoError::ArithmeticOverflow("selected stratum retention"))?,
        })
    })();
    context.release_workspace(storage)?;
    result
}

fn selected_storage(
    output: &SelectedStrata,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<u64, GeoError> {
    let mut owned = (size_of::<super::NativeBoundaryArrangement>() as u64)
        .checked_add(
            (output.fragments.capacity() as u64)
                .checked_mul(size_of::<super::NativeBoundaryFragment>() as u64)
                .ok_or(GeoError::ArithmeticOverflow("selected fragment retention"))?,
        )
        .and_then(|bytes| {
            (output.points.capacity() as u64)
                .checked_mul(size_of::<super::NativeSelectedPoint>() as u64)
                .and_then(|extra| bytes.checked_add(extra))
        })
        .ok_or(GeoError::ArithmeticOverflow("selected result retention"))?;
    for parameter in output
        .fragments
        .iter()
        .flat_map(|fragment| fragment.parameters.iter())
        .chain(output.points.iter().map(|point| &point.parameter))
    {
        metadata_work(1, context, progress)?;
        owned = owned
            .checked_add(
                parameter
                    .retained_workspace_bytes()
                    .saturating_sub(size_of::<super::SourceParameter>() as u64),
            )
            .ok_or(GeoError::ArithmeticOverflow("selected parameter retention"))?;
    }
    Ok(owned)
}

/// Whether an original edge is a structurally proved constant image. Written
/// seam aliases alone do not prove constancy of a longitude-linear full turn.
pub(super) fn is_constant(
    value: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    if let PreparedEdge::Transformed(image) = value {
        let (first, last) = image.source_endpoints();
        let operands = [
            Some(first.x()),
            Some(first.y()),
            first.z(),
            Some(last.x()),
            Some(last.y()),
            last.z(),
        ]
        .into_iter()
        .flatten()
        .collect::<SmallVec<[&Rat; 6]>>();
        let same_source =
            ExactAdmission::new(context, progress).rational(Linear, &operands, 6, || {
                Ok(first.same_planar(last) && first.z() == last.z())
            })?;
        if same_source {
            return Ok(true);
        }
        return transformed_constant(value, context, progress);
    }
    super::super::constant::constant_point(value, context, progress).map(|point| point.is_some())
}

fn transformed_constant(
    edge: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let policy = context.policy();
    let limit = policy.limits().max_precision_bits;
    let mut bits = 80.min(limit);
    loop {
        let result = super::super::with_line_refs_with_precision(
            &[edge],
            bits,
            true,
            context,
            progress,
            |lines, math, progress| {
                let evaluate = (|| {
                    let whole =
                        crate::numerical::fixed_from_bounds(&Rat::zero(), &Rat::one(), math)?;
                    let (normal, derivative) =
                        lines[0].normal(&whole, policy.limits().max_iterations, math, progress)?;
                    if normal.iter().all(|value| value.lower() == value.upper())
                        || derivative
                            .as_ref()
                            .is_some_and(|values| values.iter().all(FixedInterval::is_exact_zero))
                    {
                        return Ok(true);
                    }
                    // One strict derivative value proves a nonconstant analytic
                    // image. A finite collection of zero samples proves nothing.
                    for depth in 0..policy.limits().max_subdivision_levels.min(31) {
                        let denominator = 1_i64 << depth;
                        for numerator in 0..=denominator {
                            progress.math_poll(math)?;
                            let parameter = FixedInterval::from_i64(numerator, math)?
                                .div(&FixedInterval::from_i64(denominator, math)?, math)?;
                            let (_, derivative) = lines[0].normal(
                                &parameter,
                                policy.limits().max_iterations,
                                math,
                                progress,
                            )?;
                            if derivative.is_some_and(|values| {
                                values.iter().any(|value| {
                                    value.upper().is_negative()
                                        || (!value.lower().is_negative()
                                            && !value.lower().is_zero())
                                })
                            }) {
                                return Ok(false);
                            }
                        }
                    }
                    Err(purrdf_xsd::math::MathError::PrecisionExhausted)
                })();
                evaluate.map_err(|error| geo_math_error(&error, policy))
            },
        );
        match result {
            Err(GeoError::PrecisionExhausted { .. }) if bits < limit => {
                bits = bits.saturating_mul(2).min(limit);
            }
            result => return result,
        }
    }
}

fn active_overlap(
    overlap: &Overlap,
    address: Address,
    parameter: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<Address>, GeoError> {
    let Some(side) = overlap.pair.iter().position(|value| *value == address) else {
        return Ok(None);
    };
    let bounds = &overlap.bounds[side];
    let (first, last) = if side == 1 && overlap.reversed {
        (&bounds[1], &bounds[0])
    } else {
        (&bounds[0], &bounds[1])
    };
    let contained = ExactAdmission::new(context, progress).rational(
        RationalCompare,
        &[parameter, &first.0, &first.1, &last.0, &last.1],
        4,
        || {
            if parameter > &first.1 && parameter < &last.0 {
                Ok(true)
            } else if parameter < &first.0 || parameter > &last.1 {
                Ok(false)
            } else {
                Err(GeoError::PrecisionExhausted { bits: 0 })
            }
        },
    )?;
    Ok(contained.then_some(overlap.pair[1 - side]))
}

pub(crate) fn relation_matrix(
    a: &PreparedGeometry,
    b: &PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<IntersectionMatrix, GeoError> {
    let mut count = 0usize;
    for input in [a, b] {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        count = count
            .checked_add(input.points().len())
            .and_then(|count| count.checked_add(input.symbolic_points().len()))
            .ok_or(GeoError::ArithmeticOverflow(
                "relation graph edge inventory",
            ))?;
        for curve in input.curves().iter().chain(rings(input)) {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            count = count
                .checked_add(curve.edges().len())
                .ok_or(GeoError::ArithmeticOverflow(
                    "relation graph edge inventory",
                ))?;
        }
    }
    let reservation = (count as u64)
        .checked_mul(8192)
        // Point adapters own two original endpoint copies; polygon/curve Arcs
        // share their source. The doubled original receipts conservatively
        // admit every live clone before the combined preparation starts.
        .and_then(|bytes| {
            a.retained_workspace_bytes()
                .checked_mul(2)
                .and_then(|extra| bytes.checked_add(extra))
        })
        .and_then(|bytes| {
            b.retained_workspace_bytes()
                .checked_mul(2)
                .and_then(|extra| bytes.checked_add(extra))
        })
        .ok_or(GeoError::ArithmeticOverflow(
            "relation graph source storage",
        ))?;
    context.charge_work(
        u64::try_from(count)
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or(GeoError::ArithmeticOverflow(
                "relation edge initialization work",
            ))?,
    )?;
    context.admit_workspace(reservation)?;
    let result = (|| {
        progress.context_poll(context)?;
        prepare(a, b, context, progress)
    })()
    .and_then(|(source, layout)| {
        let base = source.retained_workspace_bytes();
        let mut inventory = Inventory {
            source: Arc::new(source),
            addresses: Vec::new(),
            cuts: Vec::new(),
            nodes: Vec::new(),
            parents: Vec::new(),
            ranks: Vec::new(),
            canonical: Vec::new(),
            overlaps: Vec::new(),
            retained: base,
        };
        let result = (|| {
            let mut goal = 32.min(context.policy().limits().max_precision_bits);
            loop {
                initialize(&mut inventory, context, progress)?;
                let contacts = node_pairs(&mut inventory, goal, context, progress);
                match contacts {
                    Err(GeoError::PrecisionExhausted { .. })
                        if goal < context.policy().limits().max_precision_bits =>
                    {
                        super::reset_inventory(&mut inventory, base, context, progress)?;
                        goal = goal
                            .saturating_mul(2)
                            .min(context.policy().limits().max_precision_bits);
                    }
                    result => {
                        result?;
                        break;
                    }
                }
            }
            witnesses(&inventory, &layout, context, progress)
        })();
        let retained = inventory.retained;
        drop(inventory);
        context.release_workspace(retained)?;
        result
    });
    context.release_workspace(reservation)?;
    result
}

fn prepare<'a>(
    a: &'a PreparedGeometry,
    b: &'a PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(PreparedGeometry, Layout<'a>), GeoError> {
    let mut retained = 0;
    let result = prepare_inner(a, b, context, progress, &mut retained);
    if let Ok((source, _)) = &result {
        // Transfer the complete immutable combined owner before releasing any
        // temporary selected-curve source allowance.
        if let Err(error) = context.admit_workspace(source.retained_workspace_bytes()) {
            drop(result);
            context.release_workspace(retained)?;
            return Err(error);
        }
    }
    context.release_workspace(retained)?;
    result
}

fn prepare_inner<'a>(
    a: &'a PreparedGeometry,
    b: &'a PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<(PreparedGeometry, Layout<'a>), GeoError> {
    let mut polygons_out = Vec::new();
    let mut curves_out = Vec::new();
    let mut layout = Layout {
        inputs: [a.region(), b.region()],
        polygon_offsets: [0; 3],
        curves: Vec::new(),
    };
    let mut polygon_capacity = 0usize;
    let mut curve_capacity = 0usize;
    for input in [a, b] {
        metadata_work(1, context, progress)?;
        polygon_capacity = polygon_capacity
            .checked_add(polygons(input.region())?.len())
            .ok_or(GeoError::ArithmeticOverflow("relation polygon capacity"))?;
        curve_capacity = curve_capacity
            .checked_add(input.curves().len())
            .and_then(|count| count.checked_add(input.points().len()))
            .and_then(|count| count.checked_add(input.symbolic_points().len()))
            .ok_or(GeoError::ArithmeticOverflow("relation curve capacity"))?;
    }
    reserve_metadata(&mut polygons_out, polygon_capacity, context, progress)?;
    reserve_metadata(&mut curves_out, curve_capacity, context, progress)?;
    reserve_metadata(&mut layout.curves, curve_capacity, context, progress)?;
    for (owner, input) in [a, b].into_iter().enumerate() {
        for polygon in polygons(input.region())? {
            polygons_out.push(if polygon.closed_support() {
                selected_support(polygon, context, progress, retained)?
            } else {
                copy_polygon(polygon, context, progress)?
            });
        }
        layout.polygon_offsets[owner + 1] = polygons_out.len();
        for curve in input.curves() {
            // PreparedCurve::clone copies only its immutable Arc handle. It
            // neither walks edges nor copies any original rational magnitude.
            metadata_work(1, context, progress)?;
            curves_out.push(selected_curve(curve, context, progress, retained)?);
            layout.curves.push((owner, Kind::Curve));
        }
        for point in input.points() {
            let source = point.source();
            let operands = [
                Some(source.x()),
                Some(source.y()),
                source.z(),
                source.m(),
                Some(point.point().longitude()),
                Some(point.point().latitude()),
            ]
            .into_iter()
            .flatten()
            .collect::<SmallVec<[&Rat; 6]>>();
            let copied = ExactAdmission::new(context, progress).rational(
                Linear,
                &operands,
                2 * operands.len() as u64,
                || Ok((point.clone(), point.clone())),
            )?;
            let mut edges = Vec::new();
            reserve_metadata(&mut edges, 1, context, progress)?;
            edges.push(PreparedEdge::SourceLinear(Box::new(SourceLinearEdge::new(
                copied.0, copied.1,
            )?)));
            curves_out.push(PreparedCurve::new(edges));
            layout.curves.push((owner, Kind::Point));
        }
        for point in input.symbolic_points() {
            // The image clone shares its source/chain Arcs but owns a reference
            // and optional exact epoch. Admit those actual copies separately.
            let image = point.original_curve();
            let cost = crate::numerical::reference_copy_cost(image.reference())?;
            progress.exact(context, cost, || Ok(()))?;
            if let Some(epoch) = image.exact_epoch() {
                ExactAdmission::new(context, progress).rational(Linear, &[epoch], 1, || Ok(()))?;
            }
            metadata_work(1, context, progress)?;
            let image = image.clone();
            let mut edges = Vec::new();
            reserve_metadata(&mut edges, 1, context, progress)?;
            edges.push(PreparedEdge::Transformed(Box::new(image)));
            curves_out.push(PreparedCurve::new(edges));
            layout.curves.push((owner, Kind::Point));
        }
    }
    let reference = crate::numerical::reference_clone(context, progress)?;
    let policy = context.remaining_child()?.policy();
    let result = {
        let mut observer = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        PreparedGeometry::from_parts_metered(
            reference,
            Vec::<PreparedCoordinate>::new(),
            curves_out,
            PreparedRegion::polygons(polygons_out),
            policy,
            &mut observer,
        )
    };
    let source = progress.absorb_nested(context, result)?;
    Ok((source, layout))
}

fn selected_curve(
    curve: &PreparedCurve,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<PreparedCurve, GeoError> {
    // Only allocate a replacement edge container after an original-law
    // selected-set certificate exists. Ordinary curves retain their Arc.
    let mut output = Vec::new();
    for (index, edge) in curve.edges().iter().enumerate() {
        metadata_work(1, context, progress)?;
        let reduced = match edge {
            PreparedEdge::Transformed(image) => image
                .selected_set_in(context, progress, retained)?
                .map(|image| PreparedEdge::Transformed(Box::new(image))),
            PreparedEdge::AzimuthLength(arc)
                if crate::atlas::point::covers_equator(arc, context, progress)? =>
            {
                Some(full_equator(context, progress, retained)?)
            }
            _ => None,
        };
        let candidate = reduced.as_ref().unwrap_or(edge);
        let meridian = if let PreparedEdge::AzimuthLength(arc) = candidate {
            crate::atlas::point::covered_meridian(arc, context, progress, retained)?
        } else {
            None
        };
        let pieces = if let Some(plane) = meridian {
            Some(full_meridian(&plane, context, progress, retained)?)
        } else if matches!(candidate, PreparedEdge::Transformed(_))
            && !is_constant(candidate, context, progress)?
            && !crate::atlas::point::image_injective(candidate, context, progress)?
        {
            Some(injective_pieces(candidate, context, progress, retained)?)
        } else {
            None
        };
        if let Some(pieces) = pieces {
            if output.is_empty() {
                reserve_metadata(&mut output, curve.edges().len(), context, progress)?;
                for edge in &curve.edges()[..index] {
                    output.push(edge.clone_admitted(context, progress, retained)?);
                }
            }
            reserve_metadata(&mut output, pieces.len(), context, progress)?;
            output.extend(pieces);
            continue;
        }
        if let Some(edge) = reduced {
            if output.is_empty() {
                reserve_metadata(&mut output, curve.edges().len(), context, progress)?;
                for edge in &curve.edges()[..index] {
                    output.push(edge.clone_admitted(context, progress, retained)?);
                }
            }
            output.push(edge);
        } else if !output.is_empty() {
            output.push(edge.clone_admitted(context, progress, retained)?);
        }
    }
    Ok(if output.is_empty() {
        curve.clone()
    } else {
        PreparedCurve::new(output)
    })
}

fn selected_support(
    polygon: &crate::PreparedPolygon,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<crate::PreparedPolygon, GeoError> {
    let bytes = polygon
        .rings()
        .len()
        .checked_mul(size_of::<PreparedCurve>())
        .and_then(|bytes| bytes.checked_add(2 * size_of::<usize>()))
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(GeoError::ArithmeticOverflow(
            "selected support curve storage",
        ))?;
    context.retain_workspace(bytes, retained)?;
    let mut curves = Vec::new();
    reserve_metadata(&mut curves, polygon.rings().len(), context, progress)?;
    for curve in polygon.rings() {
        metadata_work(1, context, progress)?;
        curves.push(selected_curve(curve, context, progress, retained)?);
    }
    let reference = crate::numerical::reference_clone(context, progress)?;
    Ok(
        crate::PreparedPolygon::from_selected_support(curves, &reference)?
            .with_interior(polygon.interior()),
    )
}

pub(super) fn selected_region(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<PreparedRegion, GeoError> {
    let bytes = polygons(region)?
        .len()
        .checked_mul(size_of::<crate::PreparedPolygon>())
        .and_then(|bytes| bytes.checked_add(2 * size_of::<usize>()))
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(GeoError::ArithmeticOverflow(
            "selected region polygon storage",
        ))?;
    context.retain_workspace(bytes, retained)?;
    let mut output = Vec::new();
    reserve_metadata(&mut output, polygons(region)?.len(), context, progress)?;
    for polygon in polygons(region)? {
        metadata_work(1, context, progress)?;
        output.push(if polygon.closed_support() {
            selected_support(polygon, context, progress, retained)?
        } else {
            copy_polygon(polygon, context, progress)?
        });
    }
    Ok(match region {
        PreparedRegion::Empty => PreparedRegion::Empty,
        PreparedRegion::Whole => PreparedRegion::Whole,
        PreparedRegion::Polygons(_) => PreparedRegion::polygons(output),
        PreparedRegion::ComplementOfPolygons(_) => PreparedRegion::polygons(output).complement(),
    })
}

fn injective_pieces(
    edge: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Vec<PreparedEdge>, GeoError> {
    let PreparedEdge::Transformed(image) = edge else {
        unreachable!("checked image")
    };
    let mut pending = purrdf_lex::walk::WorkList::<(u128, u32), 128>::with((0, 0));
    let mut output = Vec::new();
    while let Some((index, depth)) = pending.pop() {
        metadata_work(1, context, progress)?;
        let parameters = [index, index + 1].map(|numerator| {
            let cost = purrdf_xsd::integer::ExactArithmeticCost::dyadic_rational(
                u64::from(numerator.bit_width()),
                depth,
            )
            .ok_or(GeoError::ArithmeticOverflow("selected source parameter"))?;
            progress.exact(context, cost, || {
                Ok(Rat::from_dyadic_integer(
                    &crate::Int::from_u128(numerator),
                    depth,
                ))
            })
        });
        let [a, b] = parameters;
        let mut temporary = 0;
        let result = (|| {
            let piece = image.restricted_in([&a?, &b?], context, progress, &mut temporary)?;
            context.retain_workspace(size_of::<OperationImageCurve>() as u64, &mut temporary)?;
            let edge = PreparedEdge::Transformed(Box::new(piece));
            if is_constant(&edge, context, progress)?
                || crate::atlas::point::image_injective(&edge, context, progress)?
            {
                if output.len() as u64 >= context.policy().limits().max_output_elements {
                    return Err(GeoError::OutputExhausted {
                        limit: context.policy().limits().max_output_elements,
                    });
                }
                reserve_metadata(&mut output, 1, context, progress)?;
                let owners = retained
                    .checked_add(temporary)
                    .ok_or(GeoError::ArithmeticOverflow("selected panel owners"))?;
                output.push(edge);
                *retained = owners;
                temporary = 0;
                return Ok(());
            }
            drop(edge);
            if depth >= context.policy().limits().max_subdivision_levels || depth >= 127 {
                return Err(GeoError::PrecisionExhausted {
                    bits: context.policy().limits().max_precision_bits,
                });
            }
            pending.push((index * 2 + 1, depth + 1));
            pending.push((index * 2, depth + 1));
            Ok(())
        })();
        context.release_workspace(temporary)?;
        result?;
    }
    Ok(output)
}

fn full_equator(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<PreparedEdge, GeoError> {
    context.retain_workspace(size_of::<SourceLinearEdge>() as u64, retained)?;
    let mut point = |longitude| {
        let value = match context.reference().axes() {
            crate::AxisOrder::LonLat => crate::Coord::xy(Rat::from_i64(longitude), Rat::zero()),
            crate::AxisOrder::LatLon => crate::Coord::xy(Rat::zero(), Rat::from_i64(longitude)),
        };
        let cost = crate::numerical::rational_cost(RationalCompare, &[value.x(), value.y()], 4)
            .ok_or(GeoError::ArithmeticOverflow("selected equator endpoints"))?;
        progress.exact_context(context, cost, |context| {
            PreparedCoordinate::new(value, context.reference())
        })
    };
    let edge = SourceLinearEdge::new(point(-180)?, point(180)?)?;
    Ok(PreparedEdge::SourceLinear(Box::new(edge)))
}

fn full_meridian(
    plane: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Vec<PreparedEdge>, GeoError> {
    let copy = crate::numerical::rational_cost(Linear, &[plane], 1)
        .ok_or(GeoError::ArithmeticOverflow("selected meridian plane"))?;
    context.retain_workspace(copy.workspace_bytes, retained)?;
    let plane = crate::atlas::point::longitude_admitted(plane, context, progress)?;
    let mut admission = ExactAdmission::new(context, progress);
    let opposite = admission.rational_owner(
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[&plane, &Rat::from_i64(180)],
        retained,
        || plane.add(&Rat::from_i64(180)),
    )?;
    let opposite = crate::atlas::point::longitude_admitted(&opposite, context, progress)?;
    context.retain_workspace(
        (2 * size_of::<PreparedEdge>() + 2 * size_of::<SourceLinearEdge>()) as u64,
        retained,
    )?;
    let mut output = Vec::new();
    reserve_metadata(&mut output, 2, context, progress)?;
    for (longitude, start, end) in [(&plane, -90, 90), (&opposite, 90, -90)] {
        let mut point = |latitude| {
            let operands = [longitude, &Rat::from_i64(latitude)];
            let cost = crate::numerical::rational_cost(Linear, &operands, 2).ok_or(
                GeoError::ArithmeticOverflow("selected meridian source copy"),
            )?;
            context.retain_workspace(
                cost.workspace_bytes + size_of::<PreparedCoordinate>() as u64,
                retained,
            )?;
            let coordinate = progress.exact(context, cost, || {
                Ok(crate::Coord::xy(longitude.clone(), Rat::from_i64(latitude)))
            })?;
            let cost = crate::numerical::rational_cost(
                RationalCompare,
                &[coordinate.x(), coordinate.y()],
                4,
            )
            .ok_or(GeoError::ArithmeticOverflow(
                "selected meridian source range",
            ))?;
            progress.exact_context(context, cost, |context| {
                PreparedCoordinate::new(coordinate, context.reference())
            })
        };
        output.push(PreparedEdge::SourceLinear(Box::new(SourceLinearEdge::new(
            point(start)?,
            point(end)?,
        )?)));
    }
    Ok(output)
}

pub(super) fn closed_endpoints(
    edge: &PreparedEdge,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    if is_constant(edge, context, progress)? {
        return Ok(true);
    }
    let PreparedEdge::SourceLinear(line) = edge else {
        return Ok(false);
    };
    let a = line.start().point();
    let b = line.end().point();
    ExactAdmission::new(context, progress).rational(
        Linear,
        &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
        8,
        || Ok(a.same_location(b)),
    )
}

fn metadata_work(
    count: usize,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    context.charge_work(
        u64::try_from(count).map_err(|_| GeoError::ArithmeticOverflow("relation metadata walk"))?,
    )?;
    progress.context_poll(context)
}

fn incident(
    inventory: &Inventory,
    address: Address,
    parameter: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<(Address, bool)>, GeoError> {
    let capacity = inventory
        .overlaps
        .len()
        .checked_add(1)
        .ok_or(GeoError::ArithmeticOverflow("relation incident capacity"))?;
    let mut output = Vec::new();
    reserve_metadata(&mut output, capacity, context, progress)?;
    output.push((address, true));
    for overlap in &inventory.overlaps {
        if let Some(other) = active_overlap(overlap, address, parameter, context, progress)? {
            output.push((other, !overlap.reversed));
        }
    }
    Ok(output)
}

fn region_sides(
    inventory: &Inventory,
    layout: &Layout<'_>,
    address: Address,
    parameter: &Rat,
    involved: &[(Address, bool)],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<FragmentLocations, GeoError> {
    let constants = layout.inputs.map(|input| match input {
        PreparedRegion::Empty => Some(Set::Exterior),
        PreparedRegion::Whole => Some(Set::Interior),
        PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_) => None,
    });
    if let [Some(first), Some(second)] = constants {
        // No point evaluation is required to classify a constant region. In
        // particular, pure curve topology consumes the contact graph without
        // solving an unrelated continuous image at every open fragment.
        return Ok(FragmentLocations {
            faces: [[first; 2], [second; 2]],
            closed: [first, second],
        });
    }
    let geometric = region_needs_geometry(layout.inputs[0], context, progress)?
        || region_needs_geometry(layout.inputs[1], context, progress)?;
    let exact = if geometric {
        exact_fragment_point(
            edge(&inventory.source, address),
            parameter,
            context,
            progress,
        )?
    } else {
        None
    };
    let mut bits = 96.min(context.policy().limits().max_precision_bits);
    loop {
        let image = if geometric && exact.is_none() {
            Some(fragment_point(
                edge(&inventory.source, address),
                parameter,
                bits,
                context,
                progress,
            )?)
        } else {
            None
        };
        let mut output = [[Set::Exterior; 2]; 2];
        let mut closed = [Set::Exterior; 2];
        let mut complete = true;
        for (owner, row) in output.iter_mut().enumerate() {
            let input = layout.inputs[owner];
            let mut selected = [Vec::new(), Vec::new()];
            let mut selected_closed = Vec::new();
            reserve_metadata(
                &mut selected_closed,
                polygons(input)?.len(),
                context,
                progress,
            )?;
            for values in &mut selected {
                reserve_metadata(values, polygons(input)?.len(), context, progress)?;
            }
            for (polygon_index, polygon) in polygons(input)?.iter().enumerate() {
                let scans = polygon
                    .rings()
                    .len()
                    .checked_mul(
                        involved
                            .len()
                            .checked_add(1)
                            .ok_or(GeoError::ArithmeticOverflow("relation side incidence"))?,
                    )
                    .and_then(|count| count.checked_add(1))
                    .ok_or(GeoError::ArithmeticOverflow("relation side inventory"))?;
                metadata_work(scans, context, progress)?;
                let global_polygon = layout.polygon_offsets[owner] + polygon_index;
                if polygon.closed_support() {
                    // Every original pair was completely noded. An open
                    // fragment contacts this support exactly when its positive
                    // overlap incidence names an edge of the support. A support
                    // has no open face, irrespective of traversal orientation.
                    let touching = involved
                        .iter()
                        .any(|(value, _)| value.polygon == global_polygon);
                    selected[0].push(false);
                    selected[1].push(false);
                    selected_closed.push(support_location(
                        polygon,
                        touching,
                        matches!(input, PreparedRegion::ComplementOfPolygons(_)),
                    ));
                    continue;
                }
                if polygon.chart().is_some() {
                    let touching = involved
                        .iter()
                        .any(|(value, _)| value.polygon == global_polygon);
                    let location = if touching {
                        let PreparedEdge::SourceLinear(line) = edge(&inventory.source, address)
                        else {
                            complete = false;
                            break;
                        };
                        let point = exact.as_ref().expect("written representative");
                        let coordinate =
                            crate::atlas::relate::copy_point(point, context, progress)?;
                        let from = crate::atlas::relate::copy_point(
                            line.start().point(),
                            context,
                            progress,
                        )?;
                        let to = crate::atlas::relate::copy_point(
                            line.end().point(),
                            context,
                            progress,
                        )?;
                        crate::atlas::written_boundary_locations_in(
                            &coordinate,
                            &from,
                            &to,
                            core::slice::from_ref(polygon),
                            false,
                            context,
                            progress,
                        )?
                    } else {
                        let located = if let Some(point) = &exact {
                            Some(crate::atlas::polygon_location(
                                point, polygon, context, progress,
                            )?)
                        } else {
                            let region = single_polygon(polygon, context, progress)?;
                            let image = image.as_ref().expect("numerical representative");
                            crate::atlas::locate_enclosure(
                                &image[0], &image[1], &region, context, progress,
                            )?
                        };
                        let Some(location) = located.filter(|location| *location != Set::Boundary)
                        else {
                            complete = false;
                            break;
                        };
                        [location; 2]
                    };
                    let closed_location = if location == [Set::Interior; 2] {
                        Set::Interior
                    } else if location[0] != location[1] {
                        Set::Boundary
                    } else if touching {
                        // A written collapsed set can retain its original wall
                        // even when both complete transverse faces are outside.
                        crate::atlas::polygon_location(
                            exact.as_ref().expect("written representative"),
                            polygon,
                            context,
                            progress,
                        )?
                    } else {
                        location[0]
                    };
                    selected_closed.push(closed_location);
                    // region_membership applies the polygon's declared complement.
                    let flipped = polygon.interior() == crate::RegionInterior::Complement;
                    selected[0].push((location[0] == Set::Interior) ^ flipped);
                    selected[1].push((location[1] == Set::Interior) ^ flipped);
                    continue;
                }
                // A certified operation cell whose outer walls exclude the
                // whole numerical representative has an Exterior base; no ring
                // carries this fragment, so both open faces agree.
                if let Some(image) = image.as_ref()
                    && !involved
                        .iter()
                        .any(|(value, _)| value.polygon == global_polygon)
                    && crate::atlas::enclosure_outside_certified_walls(
                        polygon, &image[0], &image[1], context, progress,
                    )?
                {
                    selected[0].push(false);
                    selected[1].push(false);
                    selected_closed.push(crate::atlas::polygon_selected_location(
                        polygon,
                        Set::Exterior,
                    ));
                    continue;
                }
                let mut states = [Vec::new(), Vec::new()];
                let mut closed_states = Vec::new();
                reserve_metadata(&mut closed_states, polygon.rings().len(), context, progress)?;
                for values in &mut states {
                    reserve_metadata(values, polygon.rings().len(), context, progress)?;
                }
                for (ring_index, ring) in polygon.rings().iter().enumerate() {
                    if let Some((_, same)) = involved.iter().find(|(value, _)| {
                        value.polygon == global_polygon && value.local_ring == ring_index
                    }) {
                        states[0].push(*same);
                        states[1].push(!same);
                        closed_states.push(Set::Boundary);
                    } else {
                        let located = if let Some(point) = &exact {
                            Some(crate::atlas::oriented::locate_validated(
                                ring, point, context, progress,
                            )?)
                        } else {
                            let image = image.as_ref().expect("numerical representative");
                            crate::atlas::locate_ring_enclosure(
                                &image[0], &image[1], ring, context, progress,
                            )?
                        };
                        let Some(location) = located.filter(|location| *location != Set::Boundary)
                        else {
                            complete = false;
                            break;
                        };
                        states[0].push(location == Set::Interior);
                        states[1].push(location == Set::Interior);
                        closed_states.push(location);
                    }
                }
                if !complete {
                    break;
                }
                selected[0].push(crate::atlas::polygon_base_membership(polygon, &states[0])?);
                selected[1].push(crate::atlas::polygon_base_membership(polygon, &states[1])?);
                let base =
                    crate::atlas::ring_intersection_location(closed_states.into_iter().map(Ok))?;
                selected_closed.push(crate::atlas::polygon_selected_location(polygon, base));
            }
            if !complete {
                break;
            }
            closed[owner] = crate::atlas::raw_union_location(
                selected_closed.into_iter().map(Ok),
                matches!(input, PreparedRegion::ComplementOfPolygons(_)),
            )?
            .0;
            for side in 0..2 {
                row[side] = if crate::atlas::region_membership(input, &selected[side])? {
                    Set::Interior
                } else {
                    Set::Exterior
                };
            }
            if matches!(input, PreparedRegion::ComplementOfPolygons(_))
                && *row == [Set::Exterior; 2]
            {
                // Complete noding leaves no other wall through an open
                // fragment. Both sides outside the complement therefore prove
                // an original union interior neighbourhood, whose points the
                // closed complement removes even at multiple original walls.
                closed[owner] = Set::Exterior;
            }
        }
        if complete {
            return Ok(FragmentLocations {
                faces: output,
                closed,
            });
        }
        if bits == context.policy().limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted { bits });
        }
        bits = bits
            .saturating_mul(2)
            .min(context.policy().limits().max_precision_bits);
    }
}

fn support_location(
    polygon: &crate::PreparedPolygon,
    touching: bool,
    outer_complement: bool,
) -> Set {
    if polygon.interior() == crate::RegionInterior::Complement {
        Set::Interior
    } else if touching && !outer_complement {
        Set::Boundary
    } else {
        Set::Exterior
    }
}

fn region_needs_geometry(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let polygons = polygons(region)?;
    metadata_work(polygons.len(), context, progress)?;
    Ok(polygons.iter().any(|polygon| !polygon.closed_support()))
}

fn curve_members(layout: &Layout<'_>, involved: &[(Address, bool)], owner: usize) -> bool {
    involved.iter().any(|(address, _)| {
        address.polygon == usize::MAX
            && matches!(layout.curves[address.local_ring],(member,Kind::Curve) if member==owner)
    })
}

fn copy_polygon(
    polygon: &crate::PreparedPolygon,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<crate::PreparedPolygon, GeoError> {
    for curve in polygon.rings() {
        metadata_work(1, context, progress)?;
        if polygon.chart().is_some() {
            for edge in curve.edges() {
                metadata_work(1, context, progress)?;
                for point in edge.original_coordinates() {
                    let operands = [Some(point.x()), Some(point.y()), point.z(), point.m()]
                        .into_iter()
                        .flatten()
                        .collect::<SmallVec<[&Rat; 4]>>();
                    ExactAdmission::new(context, progress).rational(
                        Linear,
                        &operands,
                        operands.len() as u64,
                        || Ok(()),
                    )?;
                }
            }
        }
    }
    metadata_work(1, context, progress)?;
    Ok(polygon.clone())
}

fn single_polygon(
    polygon: &crate::PreparedPolygon,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<PreparedRegion, GeoError> {
    let mut polygons = Vec::new();
    reserve_metadata(&mut polygons, 1, context, progress)?;
    polygons.push(copy_polygon(polygon, context, progress)?);
    Ok(PreparedRegion::polygons(polygons))
}

fn witnesses(
    inventory: &Inventory,
    layout: &Layout<'_>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<IntersectionMatrix, GeoError> {
    witnesses_with(inventory, layout, &mut MatrixOnly, context, progress)
}

fn witnesses_with(
    inventory: &Inventory,
    layout: &Layout<'_>,
    output: &mut impl GraphOutput,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<IntersectionMatrix, GeoError> {
    let mut cuts = 0usize;
    for values in &inventory.cuts {
        metadata_work(1, context, progress)?;
        cuts = cuts
            .checked_add(values.len())
            .ok_or(GeoError::ArithmeticOverflow("relation cut inventory"))?;
    }
    let storage = inventory
        .nodes
        .len()
        .checked_mul(size_of::<NodeWitness<'_>>())
        .and_then(|bytes| {
            cuts.checked_mul(2 * size_of::<Address>())
                .and_then(|extra| bytes.checked_add(extra))
        })
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(GeoError::ArithmeticOverflow("relation witness storage"))?;
    context.admit_workspace(storage)?;
    let result = (|| {
        progress.context_poll(context)?;
        witness_graph(inventory, layout, output, context, progress)
    })();
    context.release_workspace(storage)?;
    result
}

fn witness_graph(
    inventory: &Inventory,
    layout: &Layout<'_>,
    output: &mut impl GraphOutput,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<IntersectionMatrix, GeoError> {
    let mut matrix = IntersectionMatrix::new();
    metadata_work(
        inventory.nodes.len().saturating_add(layout.curves.len()),
        context,
        progress,
    )?;
    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(inventory.nodes.len())
        .map_err(|_| GeoError::MemoryExhausted {
            limit: context.policy().limits().max_workspace_bytes,
        })?;
    nodes.resize_with(inventory.nodes.len(), NodeWitness::default);
    for (address, cuts) in inventory.addresses.iter().copied().zip(&inventory.cuts) {
        for cut in cuts {
            context.charge_work(super::node_depth_bound(inventory.parents.len()) + 1)?;
            progress.context_poll(context)?;
            let node = root(&inventory.parents, cut.node);
            let witness = &mut nodes[node];
            if witness.representative.is_none()
                || matches!(
                    edge(&inventory.source, address),
                    PreparedEdge::SourceLinear(_)
                )
            {
                witness.representative = Some((address, cut));
            }
            witness
                .incident
                .try_reserve_exact(1)
                .map_err(|_| GeoError::MemoryExhausted {
                    limit: context.policy().limits().max_workspace_bytes,
                })?;
            witness.incident.push(address);
            if address.polygon == usize::MAX {
                let (owner, kind) = layout.curves[address.local_ring];
                match kind {
                    Kind::Point => witness.point_member[owner] = true,
                    Kind::Curve => witness.curve_member[owner] = true,
                }
            }
        }
    }
    let mut fragments = 0usize;
    for (index, address) in inventory.addresses.iter().copied().enumerate() {
        if is_constant(edge(&inventory.source, address), context, progress)? {
            continue;
        }
        for ends in inventory.cuts[index].windows(2) {
            let parameter = midpoint(&ends[0], &ends[1], context, progress)?;
            let involved = incident(inventory, address, &parameter, context, progress)?;
            // Count each physical open fragment once. A written representative
            // preserves exact affine witnesses; the address breaks every remaining
            // tie. This also prevents duplicated overlaps from cancelling the
            // mod-two boundary of a selected lower-dimensional curve family.
            metadata_work(involved.len(), context, progress)?;
            let representative = involved
                .iter()
                .map(|(other, _)| {
                    (
                        !matches!(
                            edge(&inventory.source, *other),
                            PreparedEdge::SourceLinear(_)
                        ),
                        *other,
                    )
                })
                .min()
                .expect("original incident fragment");
            if representative.1 != address {
                continue;
            }
            let locations = region_sides(
                inventory, layout, address, &parameter, &involved, context, progress,
            )?;
            let faces = locations.faces;
            let mut path = [Set::Exterior; 2];
            for owner in 0..2 {
                path[owner] = if faces[owner] == [Set::Interior; 2] {
                    Set::Interior
                } else if faces[owner][0] != faces[owner][1] {
                    Set::Boundary
                } else if curve_members(layout, &involved, owner) {
                    Set::Interior
                } else {
                    let member = locations.closed[owner];
                    // Both complete transverse faces are outside. A retained
                    // closed-set fragment therefore belongs to the one-dimensional
                    // stratum, whose open points are curve interior.
                    if member == Set::Exterior {
                        Set::Exterior
                    } else {
                        Set::Interior
                    }
                };
            }
            matrix.raise(path[0], path[1], Dim::One);
            output.fragment(
                inventory,
                FragmentWitness {
                    address,
                    ends,
                    locations: &locations,
                },
                context,
                progress,
            )?;
            for side in 0..2 {
                matrix.raise(faces[0][side], faces[1][side], Dim::Two);
                output.face([faces[0][side], faces[1][side]]);
            }
            fragments = fragments
                .checked_add(1)
                .ok_or(GeoError::ArithmeticOverflow("relation open fragments"))?;
            context.charge_work(2 * super::node_depth_bound(inventory.parents.len()) + 2)?;
            progress.context_poll(context)?;
            for endpoint in ends {
                let node = &mut nodes[root(&inventory.parents, endpoint.node)];
                for owner in 0..2 {
                    node.boundary[owner] |= path[owner] == Set::Boundary;
                    node.inside[owner] |= faces[owner].contains(&Set::Interior);
                    node.outside[owner] |= faces[owner].contains(&Set::Exterior);
                    if path[owner] == Set::Interior && faces[owner] == [Set::Exterior; 2] {
                        node.lower_curve[owner] = true;
                        node.odd[owner] ^= true;
                    }
                }
            }
        }
    }
    if fragments == 0 {
        // Every original edge has independently proved constant image: a
        // nonconstant edge retains distinct 0/1 cuts and supplies a fragment.
        // Thus the complete region boundary has at most `addresses.len()`
        // physical points. N+1 distinct equatorial seeds guarantee one open
        // face, rather than silently omitting it after a finite fixed sample.
        let seeds = inventory
            .addresses
            .len()
            .checked_add(1)
            .ok_or(GeoError::ArithmeticOverflow("constant contact face seeds"))?;
        metadata_work(seeds, context, progress)?;
        let denominator = Rat::from_i64(
            i64::try_from(seeds)
                .map_err(|_| GeoError::ArithmeticOverflow("constant contact face denominator"))?,
        );
        let mut found = false;
        for index in 0..seeds {
            let mut admission = ExactAdmission::new(context, progress);
            let numerator =
                Rat::from_i64(i64::try_from(index).map_err(|_| {
                    GeoError::ArithmeticOverflow("constant contact face numerator")
                })?);
            let parameter = admission.rational(
                purrdf_xsd::integer::ExactOperation::RationalDivide,
                &[&numerator, &denominator],
                1,
                || {
                    numerator
                        .div(&denominator)
                        .ok_or_else(|| GeoError::domain("positive face seed denominator"))
                },
            )?;
            let longitude = SourceLinearEdge::interpolate_ordinate_admitted(
                &Rat::from_i64(-180),
                &Rat::from_i64(180),
                &parameter,
                &mut admission,
            )?;
            let cost = crate::numerical::rational_cost(RationalCompare, &[&longitude], 2).ok_or(
                GeoError::ArithmeticOverflow("constant face point admission"),
            )?;
            let point = progress.exact(context, cost, || LonLat::new(longitude, Rat::zero()))?;
            let first = crate::atlas::locate_validated_with_progress(
                &point,
                layout.inputs[0],
                context,
                progress,
            )?;
            let second = crate::atlas::locate_validated_with_progress(
                &point,
                layout.inputs[1],
                context,
                progress,
            )?;
            if first != Set::Boundary && second != Set::Boundary {
                matrix.raise(first, second, Dim::Two);
                output.face([first, second]);
                // Removing finitely many points from the sphere leaves one
                // connected open face. A closed region containing that face
                // contains its closure, so constant original boundary points
                // cannot remain artificial walls of its selected complement.
                for node in &mut nodes {
                    metadata_work(1, context, progress)?;
                    for (owner, face) in [first, second].into_iter().enumerate() {
                        node.inside[owner] |= face == Set::Interior;
                        node.outside[owner] |= face == Set::Exterior;
                    }
                }
                found = true;
                break;
            }
        }
        if !found {
            return Err(GeoError::PrecisionExhausted {
                bits: context.policy().limits().max_precision_bits,
            });
        }
    }
    for node in nodes {
        metadata_work(1, context, progress)?;
        let Some((address, cut)) = node.representative else {
            continue;
        };
        let mut location = [Set::Exterior; 2];
        let mut isolated = [false; 2];
        for owner in 0..2 {
            let region = if node.boundary[owner] || node.inside[owner] && node.outside[owner] {
                Set::Boundary
            } else if node.inside[owner] {
                Set::Interior
            } else if node.outside[owner]
                && matches!(
                    layout.inputs[owner],
                    PreparedRegion::ComplementOfPolygons(_)
                )
            {
                // Every incident open sector is outside this complement and
                // no selected areal boundary meets the node. The complete
                // contact graph excludes an internal union vertex; raw source
                // boundary multiplicity cannot retain an isolated point here.
                Set::Exterior
            } else {
                closed_region_location(
                    inventory,
                    layout,
                    owner,
                    ClosedQuery {
                        address,
                        bounds: [&cut.bounds.0, &cut.bounds.1],
                        incident: &node.incident,
                    },
                    context,
                    progress,
                )?
            };
            location[owner] = if node.boundary[owner] || node.inside[owner] && node.outside[owner] {
                Set::Boundary
            } else if node.inside[owner] || region == Set::Interior {
                Set::Interior
            } else if node.lower_curve[owner] {
                if node.odd[owner] {
                    Set::Boundary
                } else {
                    Set::Interior
                }
            } else if region != Set::Exterior
                || node.point_member[owner]
                || node.curve_member[owner]
            {
                // A retained node with no positive-dimensional selected incidence
                // is a point interior. Ambient closed-region Boundary is a
                // membership proof, not an inherited areal boundary label.
                Set::Interior
            } else {
                Set::Exterior
            };
            isolated[owner] = location[owner] == Set::Interior
                && !node.inside[owner]
                && !node.boundary[owner]
                && !node.lower_curve[owner];
        }
        matrix.raise(location[0], location[1], Dim::Zero);
        output.point(
            inventory,
            PointWitness {
                address,
                cut,
                isolated,
            },
            context,
            progress,
        )?;
    }
    Ok(matrix)
}

fn closed_region_location(
    inventory: &Inventory,
    layout: &Layout<'_>,
    owner: usize,
    query: ClosedQuery<'_>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Set, GeoError> {
    let region = layout.inputs[owner];
    match region {
        PreparedRegion::Empty => return Ok(Set::Exterior),
        PreparedRegion::Whole => return Ok(Set::Interior),
        PreparedRegion::Polygons(_) | PreparedRegion::ComplementOfPolygons(_) => {}
    }
    let geometric = region_needs_geometry(region, context, progress)?;
    let exact_parameter =
        ExactAdmission::new(context, progress).rational(Linear, &query.bounds, 1, || {
            Ok(query.bounds[0] == query.bounds[1])
        })?;
    let exact = if geometric && exact_parameter {
        exact_fragment_point(
            edge(&inventory.source, query.address),
            query.bounds[0],
            context,
            progress,
        )?
    } else {
        None
    };
    let limit = context.policy().limits().max_precision_bits;
    let mut bits = 96.min(limit);
    loop {
        let image = if geometric && exact.is_none() {
            Some(super::fragment_enclosure(
                edge(&inventory.source, query.address),
                query.bounds,
                bits,
                context,
                progress,
            )?)
        } else {
            None
        };
        let complementary = matches!(region, PreparedRegion::ComplementOfPolygons(_));
        let result = crate::atlas::raw_union_location(
            polygons(region)?
                .iter()
                .enumerate()
                .map(|(polygon_index, polygon)| {
                    let global_polygon = layout.polygon_offsets[owner] + polygon_index;
                    let scans = polygon
                        .rings()
                        .len()
                        .checked_mul(query.incident.len().saturating_add(1))
                        .ok_or(GeoError::ArithmeticOverflow(
                            "closed contact ring inventory",
                        ))?;
                    metadata_work(scans, context, progress)?;
                    if polygon.closed_support() {
                        return Ok(support_location(
                            polygon,
                            query
                                .incident
                                .iter()
                                .any(|address| address.polygon == global_polygon),
                            complementary,
                        ));
                    }
                    if polygon.oriented_interior().is_some() {
                        let base = crate::atlas::ring_intersection_location(
                            polygon
                                .rings()
                                .iter()
                                .enumerate()
                                .map(|(ring_index, ring)| {
                                    if query.incident.iter().any(|address| {
                                        address.polygon == global_polygon
                                            && address.local_ring == ring_index
                                    }) {
                                        return Ok(Set::Boundary);
                                    }
                                    if let Some(point) = &exact {
                                        return crate::atlas::oriented::locate_validated(
                                            ring, point, context, progress,
                                        );
                                    }
                                    let image =
                                        image.as_ref().expect("unrounded contact enclosure");
                                    crate::atlas::locate_ring_enclosure(
                                        &image[0], &image[1], ring, context, progress,
                                    )?
                                    .ok_or(GeoError::PrecisionExhausted { bits })
                                }),
                        )?;
                        Ok(crate::atlas::polygon_selected_location(polygon, base))
                    } else if let Some(point) = &exact {
                        crate::atlas::polygon_location(point, polygon, context, progress)
                    } else {
                        let single = single_polygon(polygon, context, progress)?;
                        let image = image.as_ref().expect("unrounded contact enclosure");
                        crate::atlas::locate_enclosure(
                            &image[0], &image[1], &single, context, progress,
                        )?
                        .ok_or(GeoError::PrecisionExhausted { bits })
                    }
                }),
            complementary,
        );
        match result {
            Ok((location, _)) => return Ok(location),
            Err(GeoError::PrecisionExhausted { .. }) if bits < limit => {
                bits = bits.saturating_mul(2).min(limit);
            }
            result => return result.map(|(location, _)| location),
        }
    }
}
