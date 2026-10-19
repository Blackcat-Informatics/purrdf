// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete areal images through folds. Exact labelled vertical source cells
//! partition every interior, including holes and self-crossings. Convex cells
//! are split deterministically. A whole-box Jacobian certifies injectivity when
//! ||I-AJ||_infinity<1 for an invertible central A; the mean-value inequality
//! proves this on the entire convex box, not at sampled vertices. Unresolved
//! critical cells are refined until their complete image enclosure has physical
//! diameter <=0.09m. Every emitted image box then lies within 0.1m of an actual
//! preimage, including the outward coordinate-grid guard. Original boundary
//! images retain lower-dimensional and critical contacts. All components denote
//! one closed-set union, so folds cannot create a silently chosen interior.

use super::{ImageWorker, OperationSolverLimits, PanelKind, evaluate_panel};
use crate::context::{RetainedBuilder, WorkProgress, persistent_numeric};
use crate::numerical::{
    ExactAdmission, compare_rat as compare, copy_rat as copy, fixed_from_rat, geo_math_error,
};
use crate::{Coord, CoordDim, GeoError, Geometry, GeometryBody, MetricContext, Rat, Rings};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError, RootJacobian2};

struct Cell {
    vertices: RetainedBuilder<Coord>,
    depth: u32,
    storage: u64,
}

/// The original convex source cell has an injective image in a nonpolar
/// longitude chart narrower than one turn. Only the shared whole-cell proof
/// below can create this witness; its sign is the physical image orientation
/// of the counterclockwise original source cell.
pub(crate) struct AreaCellCertificate {
    orientation: i8,
    physical_chart: bool,
    bounds: Option<([Rat; 4], Option<[Rat; 4]>, u64)>,
}

impl AreaCellCertificate {
    pub(crate) const fn orientation(&self) -> i8 {
        self.orientation
    }
    pub(crate) const fn physical_chart(&self) -> bool {
        self.physical_chart
    }
    pub(crate) fn into_bounds(self) -> Option<([Rat; 4], Option<[Rat; 4]>, u64)> {
        self.bounds
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "The bounded whole-cell proof carries inline outward walls until its one admitted immutable owner is published; boxing every visited cell would add another allocation."
)]
pub(crate) enum ExactAreaCell {
    Region(AreaCellCertificate),
    ClosedSupport,
}

#[derive(Clone, Copy)]
enum CellGoal {
    Materialization,
    Exact,
}
#[expect(
    clippy::large_enum_variant,
    reason = "Complete inner and outer proof walls stay inline until the single admitted immutable polygon publication; a per-cell box would add an allocation."
)]
enum ImageCell {
    Injective(AreaCellCertificate),
    ClosedSupport,
    CriticalBox([Rat; 4]),
    Subdivide,
}

impl super::OperationChain {
    /// Visit a complete exact image partition. The same original source strips,
    /// clipping, panel evaluator and injectivity body serve materialization.
    /// An unresolved critical cell is actually subdivided; an approximate box
    /// can never satisfy this exact preparation request.
    pub(crate) fn visit_exact_area_cells(
        &self,
        rings: &Rings,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        mut visit: impl FnMut(
            &[Coord],
            ExactAreaCell,
            &mut MetricContext,
            &mut WorkProgress<'_>,
        ) -> Result<(), GeoError>,
    ) -> Result<(), GeoError> {
        let mut worker = ImageWorker {
            chain: self,
            factor: Rat::one(),
            epoch,
            context,
            progress,
            vertices: 0,
            retained: crate::carrier::MaterializationStorage::default(),
            point_cache: purrdf_core::FastMap::default(),
            native_bytes: 0,
            cache_scalar_bytes: 0,
        };
        worker.visit_source_cells(rings, CellGoal::Exact, |worker, vertices, proof| {
            let proof = match proof {
                ImageCell::Injective(proof) => ExactAreaCell::Region(proof),
                ImageCell::ClosedSupport => ExactAreaCell::ClosedSupport,
                ImageCell::CriticalBox(_) | ImageCell::Subdivide => {
                    return Err(GeoError::ArithmeticOverflow("exact image cell proof"));
                }
            };
            visit(vertices, proof, worker.context, worker.progress)
        })
    }
}

impl ImageWorker<'_, '_, '_, '_> {
    pub(super) fn folded_area(
        &mut self,
        dim: CoordDim,
        rings: &Rings,
    ) -> Result<Geometry, GeoError> {
        if dim != CoordDim::Xy {
            // No interior height/measure interpolation is declared by a polygon
            // carrier. It cannot be invented at multiple folded preimages.
            return Err(GeoError::PrecisionExhausted {
                bits: self.context.policy().limits().max_precision_bits,
            });
        }
        self.folded_area_inner(rings)
    }
    fn folded_area_inner(&mut self, rings: &Rings) -> Result<Geometry, GeoError> {
        let mut members = RetainedBuilder::default();
        // Source rings remain in the complete image, including degenerate interiors.
        for ring in rings {
            if !ring.is_empty() {
                let line = self.line(ring)?;
                let geometry = self.make_geometry(CoordDim::Xy, GeometryBody::LineString(line))?;
                members.push_admitted(
                    geometry,
                    self.context,
                    self.progress,
                    self.retained.working_counter(),
                )?;
            }
        }
        self.visit_source_cells(
            rings,
            CellGoal::Materialization,
            |worker, vertices, proof| {
                let ring = match proof {
                    ImageCell::Injective(_) => {
                        let mut temporary = 0;
                        let result = (|| {
                            let mut ring = RetainedBuilder::default();
                            ring.reserve_admitted(
                                vertices.len().checked_add(1).ok_or(GeoError::Storage(
                                    purrdf_lex::allocation::StorageError::SizeOverflow,
                                ))?,
                                worker.context,
                                worker.progress,
                                &mut temporary,
                            )?;
                            for point in vertices.iter().chain(vertices.first()) {
                                let point = copy_coord(
                                    point,
                                    worker.context,
                                    worker.progress,
                                    Some(&mut temporary),
                                )?;
                                let point = persistent_numeric(
                                    point,
                                    worker.context,
                                    worker.progress,
                                    &mut temporary,
                                )?;
                                ring.push_reserved(point);
                            }
                            worker.line(&ring)
                        })();
                        let released = worker.context.release_workspace(temporary);
                        worker.progress.settle_result(result, released)?
                    }
                    ImageCell::CriticalBox([x0, x1, y0, y1]) => {
                        let mut ring = RetainedBuilder::default();
                        ring.reserve_admitted(
                            5,
                            worker.context,
                            worker.progress,
                            worker.retained.working_counter(),
                        )?;
                        let points = [
                            Coord::xy(
                                copy(&x0, worker.context, worker.progress)?,
                                copy(&y0, worker.context, worker.progress)?,
                            ),
                            Coord::xy(
                                copy(&x1, worker.context, worker.progress)?,
                                copy(&y0, worker.context, worker.progress)?,
                            ),
                            Coord::xy(x1, copy(&y1, worker.context, worker.progress)?),
                            Coord::xy(copy(&x0, worker.context, worker.progress)?, y1),
                            Coord::xy(x0, y0),
                        ];
                        for point in points {
                            let point = worker.retained.retain_native(
                                point,
                                worker.context,
                                worker.progress,
                            )?;
                            worker.admit_vertex(&point)?;
                            ring.push_reserved(point);
                        }
                        worker
                            .retained
                            .output_array(ring, worker.context, worker.progress)?
                    }
                    ImageCell::ClosedSupport | ImageCell::Subdivide => {
                        return Err(GeoError::ArithmeticOverflow(
                            "materialized image cell proof",
                        ));
                    }
                };
                let body = worker.ring_body(ring)?;
                let geometry = worker.make_geometry(CoordDim::Xy, body)?;
                members.push_admitted(
                    geometry,
                    worker.context,
                    worker.progress,
                    worker.retained.working_counter(),
                )
            },
        )?;
        let members = self
            .retained
            .output_array(members, self.context, self.progress)?;
        self.make_geometry(CoordDim::Xy, GeometryBody::GeometryCollection(members))
    }

    fn visit_source_cells(
        &mut self,
        rings: &Rings,
        goal: CellGoal,
        mut visit: impl FnMut(&mut Self, &[Coord], ImageCell) -> Result<(), GeoError>,
    ) -> Result<(), GeoError> {
        // Metadata scans, edge-table storage and every original result remain
        // admitted before construction, including an exact preparation caller.
        let mut edge_count = 0_u64;
        for ring in rings {
            self.context.charge_work(1 + ring.len() as u64)?;
            self.progress_poll()?;
            edge_count = edge_count
                .checked_add(ring.len().saturating_sub(1) as u64)
                .ok_or(GeoError::ArithmeticOverflow("image source edge count"))?;
        }
        let mut temporary = 0;
        self.progress_poll()?;
        let result =
            self.visit_source_cells_inner(rings, goal, edge_count, &mut temporary, &mut visit);
        let released = self.context.release_workspace(temporary);
        result.and(released)
    }

    fn visit_source_cells_inner(
        &mut self,
        rings: &Rings,
        goal: CellGoal,
        edge_count: u64,
        temporary: &mut u64,
        visit: &mut impl FnMut(&mut Self, &[Coord], ImageCell) -> Result<(), GeoError>,
    ) -> Result<(), GeoError> {
        let mut edges = RetainedBuilder::default();
        edges.reserve_admitted(
            usize::try_from(edge_count).map_err(|_| {
                GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow)
            })?,
            self.context,
            self.progress,
            temporary,
        )?;
        let mut bounds: Option<[&Rat; 4]> = None;
        for ring in rings {
            for point in ring {
                if let Some(bounds) = bounds.as_mut() {
                    for (index, value) in [
                        (0, point.x()),
                        (1, point.x()),
                        (2, point.y()),
                        (3, point.y()),
                    ] {
                        let order = compare(value, bounds[index], self.context, self.progress)?;
                        if (index % 2 == 0 && order.is_lt()) || (index % 2 == 1 && order.is_gt()) {
                            bounds[index] = value;
                        }
                    }
                } else {
                    bounds = Some([point.x(), point.x(), point.y(), point.y()]);
                }
            }
            for pair in ring.windows(2) {
                edges.push_reserved((&pair[0], &pair[1]));
            }
        }
        let Some(bounds) = bounds else {
            return Ok(());
        };
        (|| {
            let strips = crate::topology::arrangement::decompose(
                &edges,
                bounds,
                false,
                self.context,
                self.progress,
                temporary,
                |point, context, progress| {
                    crate::topology::locate_surface_admitted(
                        point,
                        rings,
                        &mut ExactAdmission::new(context, progress),
                    )
                },
            )?;
            let mut pending = RetainedBuilder::default();
            for strip in strips {
                let [lo0, lo1] = strip.lower;
                let [hi0, hi1] = strip.upper;
                let mut vertices = RetainedBuilder::default();
                vertices.reserve_admitted(4, self.context, self.progress, temporary)?;
                for point in [lo0, lo1, hi1, hi0] {
                    // The original strip's native numbers move intact; their
                    // enclosing strip allowance survives this entire iterator.
                    vertices.push_reserved(point);
                }
                let storage = u64::try_from(vertices.admitted_bytes()).map_err(|_| {
                    GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow)
                })?;
                pending.push_admitted(
                    Cell {
                        vertices,
                        depth: 0,
                        storage,
                    },
                    self.context,
                    self.progress,
                    temporary,
                )?;
                while let Some(mut cell) = pending.pop() {
                    self.context.charge_work(1)?;
                    self.progress_poll()?;
                    if matches!(goal, CellGoal::Exact) {
                        normalize_cell(&mut cell.vertices, self.context, self.progress)?;
                    }
                    let _ = cell_operands(&cell.vertices, self.context, self.progress)?;
                    let (proof, proof_bytes) = self.image_cell(&cell.vertices, goal)?;
                    *temporary = temporary
                        .checked_add(proof_bytes)
                        .ok_or(GeoError::ArithmeticOverflow("image cell bound receipt"))?;
                    match proof {
                        ImageCell::Subdivide => {
                            if cell.depth >= self.context.policy().limits().max_subdivision_levels {
                                return Err(GeoError::PrecisionExhausted {
                                    bits: self.context.policy().limits().max_precision_bits,
                                });
                            }
                            let [mut left, mut right] = split_owned(
                                &cell.vertices,
                                self.context,
                                self.progress,
                                temporary,
                                matches!(goal, CellGoal::Exact).then_some(self.chain),
                            )?;
                            if !right.vertices.is_empty() {
                                right.depth = cell.depth + 1;
                                pending.push_admitted(
                                    right,
                                    self.context,
                                    self.progress,
                                    temporary,
                                )?;
                            } else {
                                let bytes = right.storage;
                                drop(right);
                                self.context.release_retained_workspace(bytes, temporary)?;
                            }
                            if !left.vertices.is_empty() {
                                left.depth = cell.depth + 1;
                                pending.push_admitted(
                                    left,
                                    self.context,
                                    self.progress,
                                    temporary,
                                )?;
                            } else {
                                let bytes = left.storage;
                                drop(left);
                                self.context.release_retained_workspace(bytes, temporary)?;
                            }
                        }
                        proof => visit(self, &cell.vertices, proof)?,
                    }
                    let storage = cell.storage;
                    drop(cell);
                    // Numerical bound owners stay live through their consumer;
                    // its completed prepared owner has adopted its own receipt.
                    self.context
                        .release_retained_workspace(proof_bytes, temporary)?;
                    self.context
                        .release_retained_workspace(storage, temporary)?;
                }
            }
            Ok(())
        })()
    }
    fn image_cell(
        &mut self,
        vertices: &[Coord],
        goal: CellGoal,
    ) -> Result<(ImageCell, u64), GeoError> {
        let rectangle = matches!(goal, CellGoal::Exact)
            && crate::atlas::is_rectangle_cycle(vertices, self.context, self.progress)?;
        let bounds = cell_bounds(vertices, self.context, self.progress)?;
        let a = Coord::xy(
            copy(bounds[0], self.context, self.progress)?,
            copy(bounds[2], self.context, self.progress)?,
        );
        let b = Coord::xy(
            copy(bounds[1], self.context, self.progress)?,
            copy(bounds[3], self.context, self.progress)?,
        );
        let policy = self.context.policy();
        let chain = self.chain;
        let factor = &self.factor;
        let epoch = self.epoch;
        let angular = chain
            .operations()
            .last()
            .expect("nonempty chain")
            .target()
            .unit
            == super::CoordinateUnit::Degrees;
        self.context
            .charge_work((chain.operations().len() as u64).saturating_mul(3) + 8)?;
        self.progress.context_poll(self.context)?;
        let swapped = chain
            .operations()
            .last()
            .expect("nonempty chain")
            .target()
            .swapped_axes;
        let polynomial = chain
            .operations()
            .iter()
            .all(|operation| matches!(operation.model(), super::OperationModel::Polynomial2d(_)));
        let analytic = chain.operations().iter().all(|operation| {
            matches!(operation.model(), super::OperationModel::Polynomial2d(_))
                || operation.model().registered_injective_area()
        });
        let mut prefix = true;
        let mut affine_prefix = false;
        let affine = chain.operations().iter().all(|operation| {
            let original_affine = match operation.model() {
                super::OperationModel::Similarity2d(_) => true,
                super::OperationModel::Polynomial2d(model) => model.degree() <= 1,
                _ => false,
            };
            if prefix && original_affine {
                affine_prefix = true;
                true
            } else {
                prefix = false;
                operation.model().registered_injective_area()
            }
        });
        let affine = affine && affine_prefix;
        let source_swapped = chain.operations()[0].source().swapped_axes;
        let sources = [a.x(), a.y(), b.x(), b.y(), factor];
        self.context.prepare_integer_scratch_for_observed(
            chain
                .max_original_operand_bits()
                .max(epoch.map_or(0, crate::numerical::rational_operand_bits)),
            self.progress,
        )?;
        crate::numerical::with_math_for_sources_retained(
            self.context,
            self.progress,
            96,
            32768,
            &sources,
            |math, progress| {
                let mut retained_bounds = 0;
                let result = (|| {
                    let solver = OperationSolverLimits {
                        iterations: policy.limits().max_iterations,
                        subdivisions: policy.limits().max_subdivision_levels,
                        quantize_inverse: false,
                    };
                    let panel = evaluate_panel(
                        chain,
                        epoch,
                        (&a, &b),
                        None,
                        (solver, PanelKind::Image),
                        math,
                        progress,
                    )?;
                    let width = panel.coordinates[0]
                        .width(math)?
                        .add(&panel.coordinates[1].width(math)?, math)?;
                    let diameter = width.mul(&fixed_from_rat(factor, math)?, math)?;
                    let target = fixed_from_rat(
                        &Rat::parse_decimal("0.09").expect("frozen critical image diameter"),
                        math,
                    )?;
                    if matches!(goal, CellGoal::Exact) || diameter.upper() > target.lower() {
                        let differential = match evaluate_panel(
                            chain,
                            epoch,
                            (&a, &b),
                            None,
                            (solver, PanelKind::Jacobian),
                            math,
                            progress,
                        ) {
                            Ok(differential) => differential,
                            // A complete continuous image enclosure was proved
                            // above. A derivative can remain undefined across a
                            // cusp even though that image exists; subdivide its
                            // source cell until the image box meets the band.
                            Err(MathError::PrecisionExhausted) => {
                                return Ok(ImageCell::Subdivide);
                            }
                            Err(error) => return Err(error),
                        };
                        let physical_chart = if matches!(goal, CellGoal::Exact) && angular {
                            let ninety = FixedInterval::from_i64(90, math)?;
                            let south = FixedInterval::from_i64(-90, math)?;
                            let turn = FixedInterval::from_i64(360, math)?;
                            panel.coordinates[1].lower() > south.upper()
                                && panel.coordinates[1].upper() < ninety.lower()
                                && panel.coordinates[0].width(math)?.upper() < turn.lower()
                        } else {
                            false
                        };
                        let certificate =
                            |orientation,
                             math: &mut CoordinateMath,
                             progress: &mut WorkProgress<'_>,
                             retained: &mut u64| {
                                let endpoints = if rectangle
                                    && physical_chart
                                    && differential
                                        .jacobian
                                        .as_ref()
                                        .is_some_and(weak_diagonal_monotonicity)
                                {
                                    // Exact zero cross derivatives make the map a
                                    // Cartesian product of two one-variable maps.
                                    // The enclosing branch has already certified
                                    // injectivity, so both maps are strictly monotone.
                                    // Opposite original corners bound both endpoint
                                    // values without quantizing either coordinate.
                                    Some([
                                        evaluate_panel(
                                            chain,
                                            epoch,
                                            (&a, &a),
                                            None,
                                            (solver, PanelKind::Image),
                                            math,
                                            progress,
                                        )?,
                                        evaluate_panel(
                                            chain,
                                            epoch,
                                            (&b, &b),
                                            None,
                                            (solver, PanelKind::Image),
                                            math,
                                            progress,
                                        )?,
                                    ])
                                } else {
                                    None
                                };
                                cell_certificate(
                                    orientation,
                                    (physical_chart, goal),
                                    &panel.coordinates[..2],
                                    endpoints.as_ref().map(|ends| {
                                        [&ends[0].coordinates[..2], &ends[1].coordinates[..2]]
                                    }),
                                    math,
                                    progress,
                                    retained,
                                )
                            };
                        if let Some(jac) = differential.jacobian.as_ref() {
                            // If one original source direction is identically
                            // annihilated, the map depends on the other only.
                            // A convex cell's boundary projects onto its entire
                            // interval, so that boundary is its complete image.
                            // The same holds for a rank-lost affine map: every
                            // affine fibre meeting a convex cell meets its
                            // boundary. This is a complete image proof, not an
                            // assumption that zero determinant means constant.
                            let annihilated = [0, 1].into_iter().any(|column| {
                                [0, 1]
                                    .into_iter()
                                    .all(|row| jac[row][column].is_exact_zero())
                            });
                            let rank_lost = if matches!(goal, CellGoal::Exact) && affine {
                                jac[0][0]
                                    .mul(&jac[1][1], math)?
                                    .sub(&jac[0][1].mul(&jac[1][0], math)?, math)?
                                    .is_exact_zero()
                            } else {
                                false
                            };
                            if matches!(goal, CellGoal::Exact) && (annihilated || rank_lost) {
                                return Ok(ImageCell::ClosedSupport);
                            }
                            if let Some(orientation) = certify_injective_box(jac, math)?
                                && (!matches!(goal, CellGoal::Exact) || physical_chart)
                            {
                                return Ok(ImageCell::Injective(certificate(
                                    if source_swapped {
                                        -orientation
                                    } else {
                                        orientation
                                    },
                                    math,
                                    progress,
                                    &mut retained_bounds,
                                )?));
                            }
                        }
                        if (polynomial || (matches!(goal, CellGoal::Exact) && analytic))
                            && differential
                                .jacobian
                                .as_ref()
                                .is_some_and(weak_diagonal_monotonicity)
                        {
                            let coordinate = [(a.x(), b.x()), (a.y(), b.y())].map(|(lo, hi)| {
                                let sum = crate::numerical::math_exact_rational(
                                    ExactOperation::RationalAdd,
                                    &[lo, hi],
                                    math,
                                    progress,
                                    |values| values.add(lo, hi),
                                )?;
                                let two = Rat::from_i64(2);
                                crate::numerical::math_exact_rational(
                                    ExactOperation::RationalDivide,
                                    &[&sum, &two],
                                    math,
                                    progress,
                                    |values| {
                                        Ok(values.divide(&(sum), &two)?.expect("positive divisor"))
                                    },
                                )
                            });
                            let [x, y] = coordinate;
                            let center = Coord::xy(x?, y?);
                            let central = evaluate_panel(
                                chain,
                                epoch,
                                (&center, &center),
                                None,
                                (solver, PanelKind::Jacobian),
                                math,
                                progress,
                            )?;
                            if let Some(jac) = central.jacobian.as_ref()
                                && (!matches!(goal, CellGoal::Exact) || physical_chart)
                                && [0, 1]
                                    .into_iter()
                                    .all(|axis| !contains_zero(&jac[axis][axis]))
                            {
                                let negative = jac[0][0].upper().is_negative()
                                    ^ jac[1][1].upper().is_negative()
                                    ^ source_swapped;
                                return Ok(ImageCell::Injective(certificate(
                                    if negative { -1 } else { 1 },
                                    math,
                                    progress,
                                    &mut retained_bounds,
                                )?));
                            }
                        }
                        return Ok(ImageCell::Subdivide);
                    }
                    let places = if angular { 15 } else { 6 };
                    let mut output = purrdf_core::SmallVec::<[Rat; 4]>::default();
                    let unit = purrdf_xsd::integer::Int::from_i128(1);
                    for coordinate in &panel.coordinates[..2] {
                        let (lower, upper) = coordinate.round_decimal(places, math)?;
                        let cost = ExactArithmeticCost::for_operation(
                            ExactOperation::Linear,
                            lower.bit_len().max(upper.bit_len()),
                            2,
                        )
                        .ok_or(MathError::WorkExhausted)?;
                        math.admit_exact_cost(cost)?;
                        progress.math_poll(math)?;
                        output.push(crate::numerical::math_quantized_decimal(
                            &lower.sub(&unit),
                            places,
                            math,
                            progress,
                        )?);
                        output.push(crate::numerical::math_quantized_decimal(
                            &upper.add(&unit),
                            places,
                            math,
                            progress,
                        )?);
                    }
                    // The complete outward box must fit the total physical band;
                    // very large custom axes can require a finer admitted grid.
                    let width_x = fixed_from_rat(&output[1], math)?
                        .sub(&fixed_from_rat(&output[0], math)?, math)?;
                    let width_y = fixed_from_rat(&output[3], math)?
                        .sub(&fixed_from_rat(&output[2], math)?, math)?;
                    let full = width_x
                        .add(&width_y, math)?
                        .mul(&fixed_from_rat(factor, math)?, math)?;
                    let bound = fixed_from_rat(
                        &Rat::parse_decimal("0.1").expect("frozen image certificate"),
                        math,
                    )?;
                    if full.upper() > bound.lower() {
                        return Err(MathError::PrecisionExhausted);
                    }
                    let mut output = output.into_iter();
                    let [mut x0, mut x1, mut y0, mut y1]: [Rat; 4] = core::array::from_fn(|_| {
                        output.next().expect("two complete coordinate bounds")
                    });
                    if swapped {
                        core::mem::swap(&mut x0, &mut y0);
                        core::mem::swap(&mut x1, &mut y1);
                    }
                    Ok(ImageCell::CriticalBox([x0, x1, y0, y1]))
                })();
                result
                    .map(|cell| (cell, retained_bounds))
                    .map_err(|error| geo_math_error(&error, policy))
            },
        )
    }
}
fn contains_zero(value: &FixedInterval) -> bool {
    (value.lower().is_negative() || value.lower().is_zero()) && !value.upper().is_negative()
}

/// Retain outward walls only for a complete cell in the canonical nonpolar
/// angular chart. These proof bounds do not replace its original image curves.
fn cell_certificate(
    orientation: i8,
    (physical_chart, goal): (bool, CellGoal),
    coordinates: &[FixedInterval],
    endpoints: Option<[&[FixedInterval]; 2]>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<AreaCellCertificate, MathError> {
    let bounds = if matches!(goal, CellGoal::Exact) && physical_chart {
        let west_limit = FixedInterval::from_i64(-180, math)?;
        let east_limit = FixedInterval::from_i64(180, math)?;
        // A seam-crossing or pole-reaching chart never supplies this cache.
        // Its original complete curve membership remains authoritative.
        if coordinates[0].lower() > west_limit.upper()
            && coordinates[0].upper() < east_limit.lower()
        {
            let (west, east) = crate::numerical::exact_bounds_in(&coordinates[0], math)?;
            let (south, north) = crate::numerical::exact_bounds_in(&coordinates[1], math)?;
            let (walls, bytes) =
                crate::numerical::math_share_rationals([west, south, east, north], math, progress)?;
            let inner = if let Some([first, last]) = endpoints {
                let mut inner = purrdf_core::SmallVec::<[Rat; 4]>::default();
                for axis in 0..2 {
                    let (lower, upper) = if first[axis].upper() < last[axis].lower() {
                        (&first[axis], &last[axis])
                    } else if last[axis].upper() < first[axis].lower() {
                        (&last[axis], &first[axis])
                    } else {
                        break;
                    };
                    let (_, low) = crate::numerical::exact_bounds_in(lower, math)?;
                    let (high, _) = crate::numerical::exact_bounds_in(upper, math)?;
                    inner.push(low);
                    inner.push(high);
                }
                if inner.len() == 4 {
                    let mut values = inner.into_iter();
                    let [west, east, south, north] =
                        core::array::from_fn(|_| values.next().expect("four inner walls"));
                    let (inner, bytes) = crate::numerical::math_share_rationals(
                        [west, south, east, north],
                        math,
                        progress,
                    )?;
                    Some((inner, bytes))
                } else {
                    None
                }
            } else {
                None
            };
            *retained = bytes
                .checked_add(inner.as_ref().map_or(0, |(_, bytes)| *bytes))
                .ok_or(MathError::WorkspaceExhausted)?;
            Some((walls, inner.map(|(walls, _)| walls), *retained))
        } else {
            None
        }
    } else {
        None
    };
    Ok(AreaCellCertificate {
        orientation,
        physical_chart,
        bounds,
    })
}

/// Exact zero off-diagonals separate the analytic coordinates on the connected
/// box. A derivative of one sign is monotone; a certified nonzero derivative
/// at an actual point proves nonconstancy. Analytic nonconstancy then proves
/// strict monotonicity even when the derivative vanishes at a boundary. The
/// caller checks this actual witness, rather than a midpoint of derivative bounds.
fn weak_diagonal_monotonicity(jac: &RootJacobian2) -> bool {
    [0, 1].into_iter().all(|axis| {
        let cross = &jac[axis][1 - axis];
        cross.lower().is_zero()
            && cross.upper().is_zero()
            && (!jac[axis][axis].lower().is_negative()
                || jac[axis][axis].upper().is_negative()
                || jac[axis][axis].upper().is_zero())
    })
}
fn certify_injective_box(
    jac: &RootJacobian2,
    math: &mut CoordinateMath,
) -> Result<Option<i8>, MathError> {
    let determinant = jac[0][0]
        .mul(&jac[1][1], math)?
        .sub(&jac[0][1].mul(&jac[1][0], math)?, math)?;
    if (determinant.lower().is_negative() || determinant.lower().is_zero())
        && !determinant.upper().is_negative()
    {
        return Ok(None);
    }
    let midpoint = [
        [jac[0][0].midpoint(math)?, jac[0][1].midpoint(math)?],
        [jac[1][0].midpoint(math)?, jac[1][1].midpoint(math)?],
    ];
    let inverse = match purrdf_xsd::math::invert_jacobian2(&midpoint, math) {
        Ok(value) => value,
        Err(MathError::PrecisionExhausted) => return Ok(None),
        Err(error) => return Err(error),
    };
    let product = purrdf_xsd::math::compose_jacobians2(&inverse, jac, math)?;
    let one = FixedInterval::from_i64(1, math)?;
    for (row, entries) in product.iter().enumerate() {
        let mut norm = FixedInterval::from_i64(0, math)?;
        for (column, entry) in entries.iter().enumerate() {
            let identity = FixedInterval::from_i64(i64::from(row == column), math)?;
            norm = norm.add(&identity.sub(entry, math)?.abs(math)?, math)?;
        }
        if norm.upper() >= one.lower() {
            return Ok(None);
        }
    }
    Ok(Some(if determinant.upper().is_negative() {
        -1
    } else {
        1
    }))
}
fn cell_bounds<'a>(
    vertices: &'a [Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<[&'a Rat; 4], GeoError> {
    let first = vertices.first().expect("nonempty original convex cell");
    let mut bounds = [first.x(), first.x(), first.y(), first.y()];
    for point in &vertices[1..] {
        for (index, value) in [
            (0, point.x()),
            (1, point.x()),
            (2, point.y()),
            (3, point.y()),
        ] {
            let order = compare(value, bounds[index], context, progress)?;
            if (index % 2 == 0 && order.is_lt()) || (index % 2 == 1 && order.is_gt()) {
                bounds[index] = value;
            }
        }
    }
    Ok(bounds)
}
fn cell_operands<'a>(
    vertices: &'a [Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<purrdf_core::SmallVec<[&'a Rat; 16]>, GeoError> {
    // The original arrangement produces convex quadrilaterals. Subsequent
    // x/y cuts add at most the four supporting walls of an axis-aligned box,
    // so every cell has at most eight vertices. Check that producer invariant
    // before collecting its sixteen borrowed ordinates: this never spills.
    if vertices.len() > 8 {
        return Err(GeoError::ArithmeticOverflow("source cell vertex invariant"));
    }
    context.charge_work(vertices.len() as u64 * 2)?;
    progress.context_poll(context)?;
    Ok(vertices
        .iter()
        .flat_map(|point| [point.x(), point.y()])
        .collect())
}

fn split_owned(
    vertices: &[Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
    critical_chain: Option<&super::OperationChain>,
) -> Result<[Cell; 2], GeoError> {
    let _ = cell_operands(vertices, context, progress)?;
    let mut temporary = 0;
    let mut storage = [0_u64; 2];
    let result = split(
        vertices,
        context,
        progress,
        &mut temporary,
        &mut storage,
        critical_chain,
    );
    // The producer destroys cut parameters and partial output arrays before
    // either allowance is refunded. An original failure always wins cleanup.
    let released = context.release_workspace(temporary);
    let result = result.and_then(|halves| released.map(|()| halves));
    match result {
        Err(error) => {
            for bytes in storage {
                let _ = context.release_workspace(bytes);
            }
            Err(error)
        }
        Ok([left, right]) => {
            let next = storage[0]
                .checked_add(storage[1])
                .and_then(|bytes| retained.checked_add(bytes));
            let Some(next) = next else {
                drop(left);
                drop(right);
                for bytes in storage {
                    let _ = context.release_workspace(bytes);
                }
                return Err(GeoError::ArithmeticOverflow("source cell owner transfer"));
            };
            *retained = next;
            Ok([
                Cell {
                    vertices: left,
                    depth: 0,
                    storage: storage[0],
                },
                Cell {
                    vertices: right,
                    depth: 0,
                    storage: storage[1],
                },
            ])
        }
    }
}

fn split(
    vertices: &[Coord],
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
    storage: &mut [u64; 2],
    critical_chain: Option<&super::OperationChain>,
) -> Result<[RetainedBuilder<Coord>; 2], GeoError> {
    let bounds = cell_bounds(vertices, context, progress)?;
    let critical = if let Some(chain) = critical_chain
        && let super::OperationModel::Polynomial2d(model) = chain.operations()[0].model()
    {
        let swapped = chain.operations()[0].source().swapped_axes;
        let mut critical = None;
        for axis in 0..2 {
            if let Some(cut) = model.axis_quadratic_critical(axis, context, progress, retained)? {
                let source_axis = axis ^ usize::from(swapped);
                if compare(&cut, bounds[2 * source_axis], context, progress)?.is_gt()
                    && compare(&cut, bounds[2 * source_axis + 1], context, progress)?.is_lt()
                {
                    critical = Some((source_axis, cut));
                    break;
                }
            }
        }
        critical
    } else {
        None
    };
    let mut admission = ExactAdmission::new(context, progress);
    let (axis, cut) = if let Some(critical) = critical {
        critical
    } else {
        let dx = admission.rational_owner(
            ExactOperation::RationalAdd,
            &[bounds[1], bounds[0]],
            retained,
            |values| values.subtract(bounds[1], bounds[0]),
        )?;
        let dy = admission.rational_owner(
            ExactOperation::RationalAdd,
            &[bounds[3], bounds[2]],
            retained,
            |values| values.subtract(bounds[3], bounds[2]),
        )?;
        let axis = usize::from(admission.compare(&dy, &dx)?.is_gt());
        let start = 2 * axis;
        let sum = admission.rational_owner(
            ExactOperation::RationalAdd,
            &[bounds[start], bounds[start + 1]],
            retained,
            |values| values.add(bounds[start], bounds[start + 1]),
        )?;
        let two = Rat::from_i64(2);
        let cut = admission.rational_owner(
            ExactOperation::RationalDivide,
            &[&sum, &two],
            retained,
            |values| Ok(values.divide(&(sum), &two)?.expect("positive divisor")),
        )?;
        (axis, cut)
    };
    let mut halves: [RetainedBuilder<Coord>; 2] =
        core::array::from_fn(|_| RetainedBuilder::default());
    let capacity = vertices.len().checked_add(2).ok_or(GeoError::Storage(
        purrdf_lex::allocation::StorageError::SizeOverflow,
    ))?;
    for (half, bytes) in halves.iter_mut().zip(storage.iter_mut()) {
        half.reserve_admitted(capacity, context, progress, bytes)?;
    }
    for (index, a) in vertices.iter().enumerate() {
        let b = &vertices[(index + 1) % vertices.len()];
        let (oa, ob) = if axis == 0 {
            (a.x(), b.x())
        } else {
            (a.y(), b.y())
        };
        let ca = compare(oa, &cut, context, progress)?;
        let cb = compare(ob, &cut, context, progress)?;
        if ca.is_le() {
            let point = copy_coord(a, context, progress, Some(retained))?;
            halves[0].push_reserved(persistent_numeric(
                point,
                context,
                progress,
                &mut storage[0],
            )?);
        }
        if ca.is_ge() {
            let point = copy_coord(a, context, progress, Some(retained))?;
            halves[1].push_reserved(persistent_numeric(
                point,
                context,
                progress,
                &mut storage[1],
            )?);
        }
        if (ca.is_lt() && cb.is_gt()) || (ca.is_gt() && cb.is_lt()) {
            let mut admission = ExactAdmission::new(context, progress);
            let offset = admission.rational_owner(
                ExactOperation::RationalAdd,
                &[&cut, oa],
                retained,
                |values| values.subtract(&(cut), oa),
            )?;
            let width = admission.rational_owner(
                ExactOperation::RationalAdd,
                &[ob, oa],
                retained,
                |values| values.subtract(ob, oa),
            )?;
            let t = admission.rational_owner(
                ExactOperation::RationalDivide,
                &[&offset, &width],
                retained,
                |values| Ok(values.divide(&(offset), &width)?.expect("strict crossing")),
            )?;
            let point = crate::SourceLinearEdge::interpolate_coord_retained(
                a,
                b,
                &t,
                &mut admission,
                retained,
            )?;
            let first = copy_coord(&point, context, progress, Some(retained))?;
            halves[0].push_reserved(persistent_numeric(
                first,
                context,
                progress,
                &mut storage[0],
            )?);
            halves[1].push_reserved(persistent_numeric(
                point,
                context,
                progress,
                &mut storage[1],
            )?);
        }
    }
    Ok(halves)
}
fn copy_coord(
    point: &Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: Option<&mut u64>,
) -> Result<Coord, GeoError> {
    let mut admission = ExactAdmission::new(context, progress);
    let body = |values: &crate::numerical::ExactValues<'_>| {
        Ok(Coord::xy(
            values.original(point.x())?,
            values.original(point.y())?,
        ))
    };
    if let Some(retained) = retained {
        admission.rational_owner_counted(
            ExactOperation::Linear,
            &[point.x(), point.y()],
            2,
            retained,
            body,
        )
    } else {
        admission.expression(ExactOperation::Linear, &[point.x(), point.y()], 2, body)
    }
}

fn normalize_cell(
    vertices: &mut RetainedBuilder<Coord>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    vertices.dedup_by(|last, point| {
        Ok::<_, GeoError>(
            compare(last.x(), point.x(), context, progress)?.is_eq()
                && compare(last.y(), point.y(), context, progress)?.is_eq(),
        )
    })?;
    if vertices.len() > 1 {
        let first = &vertices[0];
        let last = vertices.last().expect("nonempty cell");
        if compare(first.x(), last.x(), context, progress)?.is_eq()
            && compare(first.y(), last.y(), context, progress)?.is_eq()
        {
            vertices.pop();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionLimits, ExecutionPolicy, GeographicReference, Int};

    #[test]
    fn jordan_witness_refusal_retains_original_certificate_comparisons() {
        let curve = crate::PreparedCurve::from_source(
            &[
                Coord::xy(Rat::zero(), Rat::zero()),
                Coord::xy(Rat::one(), Rat::one()),
            ],
            &GeographicReference::wgs84(),
        )
        .unwrap();
        for (physical_chart, orientation, reversed, empty) in [
            (false, -1, false, false),
            (true, -1, false, false),
            (true, 1, true, false),
            (true, 1, false, true),
        ] {
            let certificate = AreaCellCertificate {
                physical_chart,
                orientation,
                bounds: None,
            };
            let input = if empty {
                crate::PreparedCurve::new(Vec::new())
            } else {
                curve.clone()
            };
            let count = input.edges().len();
            let error = crate::atlas::oriented::OrientedRing::from_operation_cell(
                input,
                &certificate,
                reversed,
                &mut WorkProgress::integer(None),
            )
            .unwrap_err();
            assert!(
                matches!(error.original(), GeoError::SourcePreparation(cause)
                if **cause == crate::SourcePreparationError::JordanCertificate { physical_chart, orientation, reversed, edge_count: count })
            );
            assert_eq!(error.detail(), "operation cell Jordan certificate mismatch");
            assert!(!error.is_expression_error());
            assert_eq!(
                purrdf_testkit::error_capture::Capture::read(error.original())
                    .child("cause")
                    .code,
                "source-preparation-jordan-certificate"
            );
        }
    }

    #[test]
    fn generated_source_cells_retain_actual_wide_owners_and_refuse_one_byte_below_peak() {
        let small = Rat::new(Int::one(), Int::one().shl(255).add(&Int::one())).unwrap();
        let vertices = vec![
            Coord::xy(Rat::zero(), Rat::zero()),
            Coord::xy(Rat::one(), small.clone()),
            Coord::xy(
                Rat::new(Int::one(), Int::from_i64(2)).unwrap(),
                Rat::new(Int::from_i64(3), Int::from_i64(2)).unwrap(),
            ),
            Coord::xy(Rat::zero(), Rat::from_i64(2)),
        ];
        // Intersect the independent written line with y=1: its exact x is
        // (2-small)/(3-2*small), and remains a genuinely wide rational.
        let crossing_x = Rat::from_i64(2)
            .sub(&small)
            .div(&Rat::from_i64(3).sub(&small.mul(&Rat::from_i64(2))))
            .unwrap();
        let crossing = Coord::xy(crossing_x, Rat::one());
        let publication = WorkProgress::new(None);
        let source_bytes =
            crate::context::persistent_numeric_bytes(&vertices[1], &publication).unwrap();
        let crossing_bytes =
            crate::context::persistent_numeric_bytes(&crossing, &publication).unwrap();
        assert!(source_bytes > 0);
        assert!(crossing_bytes > 0);
        let limits = ExecutionLimits {
            max_work_items: 100_000_000,
            ..ExecutionLimits::GEOMETRY
        };
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        context.set_retained_workspace(4096).unwrap();
        context.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        let mut retained = 0;
        let cells =
            split_owned(&vertices, &mut context, &mut progress, &mut retained, None).unwrap();
        let mut actual = 0;
        let mut limb_owners = 0;
        assert_eq!(
            &*cells[0].vertices,
            &[
                Coord::xy(Rat::zero(), Rat::zero()),
                vertices[1].clone(),
                crossing.clone(),
                Coord::xy(Rat::zero(), Rat::one()),
            ]
        );
        assert_eq!(
            &*cells[1].vertices,
            &[
                crossing,
                vertices[2].clone(),
                vertices[3].clone(),
                Coord::xy(Rat::zero(), Rat::one()),
            ]
        );
        for (index, cell) in cells.iter().enumerate() {
            let storage = crate::carrier::owned_coordinates_storage(
                &cell.vertices,
                &mut context,
                &mut progress,
            )
            .unwrap();
            limb_owners += storage.bytes();
            // The independent unowned source and intersection above quote
            // their original publication layouts. Completed immutable values
            // share their already admitted owners, so a second detachment
            // quote on the output would correctly report zero new bytes.
            let bytes = u64::try_from(cell.vertices.admitted_bytes()).unwrap()
                + crossing_bytes
                + if index == 0 { source_bytes } else { 0 };
            assert_eq!(cell.storage, bytes);
            actual += bytes;
        }
        assert!(limb_owners > 0, "original wide edge intersections survive");
        assert_eq!(retained, actual);
        let peak = context.workspace_peak();
        drop(cells);
        context
            .release_retained_workspace(actual, &mut retained)
            .unwrap();
        assert_eq!(retained, 0);
        assert_eq!(
            context.current_workspace_bytes(),
            context.retained_workspace_bytes()
        );
        assert!(context.retained_workspace_bytes() >= 4096);

        let mut limited = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(ExecutionLimits {
                max_workspace_bytes: peak - 1,
                ..limits
            })
            .unwrap(),
        )
        .unwrap();
        limited.set_retained_workspace(4096).unwrap();
        limited.begin(1).unwrap();
        let mut progress = WorkProgress::new(None);
        let mut retained = 0;
        let error = split_owned(&vertices, &mut limited, &mut progress, &mut retained, None)
            .err()
            .unwrap();
        assert!(matches!(error, GeoError::MemoryExhausted { .. }));
        limited
            .release_retained_workspace(retained, &mut retained)
            .unwrap();
        assert_eq!(
            limited.current_workspace_bytes(),
            limited.retained_workspace_bytes()
        );
        assert!(limited.retained_workspace_bytes() >= 4096);
    }
}
