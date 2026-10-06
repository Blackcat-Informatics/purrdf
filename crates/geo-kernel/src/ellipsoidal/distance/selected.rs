// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original selected parameter domains. RN24 is a deterministic estimate
//! sample law; physical witnesses retain and refine the true cut family.

use super::Primitive;
use crate::atlas::arcs::arrangement::{
    NativeBoundaryFragment, NativeSelectedPoint, SourceParameter,
};
use crate::context::WorkProgress;
use crate::ellipsoidal::native::Parameters;
use crate::numerical::ExactAdmission;
use crate::{
    GeoError, LonLat, MetricContext, MetricWorkObserver, PreparedEdge, Rat, SourceLinearEdge,
};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};

pub(super) struct PointSample {
    pub(super) point: LonLat,
    pub(super) error: Rat,
    pub(super) storage: u64,
}

impl PointSample {
    pub(super) fn ordinary((point, error): (LonLat, Rat)) -> Self {
        Self {
            point,
            error,
            storage: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct SelectedDomain<'a> {
    edge: &'a PreparedEdge,
    original: [&'a SourceParameter; 2],
    samples: &'a Parameters,
    singleton: bool,
}

impl<'a> SelectedDomain<'a> {
    pub(super) fn fragment(fragment: &'a NativeBoundaryFragment, samples: &'a Parameters) -> Self {
        Self {
            edge: fragment.original_edge(),
            original: fragment.parameters().each_ref(),
            samples,
            singleton: false,
        }
    }

    pub(super) fn point_domain(point: &'a NativeSelectedPoint, samples: &'a Parameters) -> Self {
        Self {
            edge: point.original_edge(),
            original: [point.parameter(); 2],
            samples,
            singleton: true,
        }
    }

    pub(super) fn length(
        self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Rat, GeoError> {
        if self.singleton {
            return Ok(Rat::zero());
        }
        let speed = Primitive::Edge(self.edge).length(context, progress)?;
        let errors = self.samples.endpoint_errors();
        let mut admission = ExactAdmission::new(context, progress);
        let width = admission.rational(
            ExactOperation::RationalAdd,
            &self.samples.endpoints.each_ref(),
            1,
            || Ok(self.samples.endpoints[1].sub(&self.samples.endpoints[0])),
        )?;
        let error = admission.rational(ExactOperation::RationalAdd, &errors, 1, || {
            Ok(errors[0].add(errors[1]))
        })?;
        let width =
            admission.rational(ExactOperation::RationalAdd, &[&width, &error], 1, || {
                Ok(width.add(&error))
            })?;
        admission.rational(
            ExactOperation::RationalMultiply,
            &[&speed, &width],
            1,
            || Ok(speed.mul(&width)),
        )
    }

    pub(super) fn point(
        self,
        parameter: &Rat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
        physical: bool,
        bits: u32,
    ) -> Result<PointSample, GeoError> {
        let mut progress = WorkProgress::new(Some(observer));
        progress.context_poll(context)?;
        let mut retained = 0;
        let result = (|| {
            progress.context_poll(context)?;
            let mut refined = [None, None];
            if physical {
                for (index, original) in self.original.iter().enumerate() {
                    let (lower, upper) = original.bounds();
                    if !ExactAdmission::new(context, &mut progress)
                        .compare(lower, upper)?
                        .is_eq()
                    {
                        let bytes = original.refinement_storage_bound(bits)?;
                        context.retain_workspace(bytes, &mut retained)?;
                        progress.context_poll(context)?;
                        refined[index] = Some(original.refined_in(bits, context, &mut progress)?);
                    }
                }
            }
            let bounds = [0, 1].map(|index| {
                refined[index]
                    .as_ref()
                    .unwrap_or(self.original[index])
                    .bounds()
            });
            let mut admission = ExactAdmission::new(context, &mut progress);
            let low = SourceLinearEdge::interpolate_ordinate_retained(
                bounds[0].0,
                bounds[1].0,
                parameter,
                &mut admission,
                &mut retained,
            )?;
            let high = SourceLinearEdge::interpolate_ordinate_retained(
                bounds[0].1,
                bounds[1].1,
                parameter,
                &mut admission,
                &mut retained,
            )?;
            let half =
                Rat::new(crate::Int::one(), crate::Int::from_u64(2)).expect("positive divisor");
            let sample = if physical {
                SourceLinearEdge::interpolate_ordinate_retained(
                    &low,
                    &high,
                    &half,
                    &mut admission,
                    &mut retained,
                )?
            } else {
                SourceLinearEdge::interpolate_ordinate_retained(
                    &self.samples.endpoints[0],
                    &self.samples.endpoints[1],
                    parameter,
                    &mut admission,
                    &mut retained,
                )?
            };
            let (left, _) = admission.rational_retained(
                ExactOperation::RationalAdd,
                &[&sample, &low],
                &mut retained,
                || sample.sub(&low).abs(),
            )?;
            let (right, _) = admission.rational_retained(
                ExactOperation::RationalAdd,
                &[&high, &sample],
                &mut retained,
                || high.sub(&sample).abs(),
            )?;
            let parameter_error = if admission.compare(&left, &right)?.is_lt() {
                right
            } else {
                left
            };
            let speed = Primitive::Edge(self.edge).length(context, &mut progress)?;
            let (error, _) = ExactAdmission::new(context, &mut progress).rational_retained(
                ExactOperation::RationalMultiply,
                &[&speed, &parameter_error],
                &mut retained,
                || speed.mul(&parameter_error),
            )?;
            if !physical
                && ExactAdmission::new(context, &mut progress)
                    .compare(&error, &super::decimal("0.000001"))?
                    .is_gt()
            {
                return Err(GeoError::PrecisionExhausted {
                    bits: context.policy().limits().max_precision_bits,
                });
            }
            // Use the actual completed sample's original numerator/denominator
            // width, including any refined implicit cut. Affine coordinates
            // have at most four input widths plus carries after difference,
            // product and addition; numerical image coordinates have the
            // requested fixed precision plus degree/alias bits. Every exact
            // temporary above already keeps its own operation allowance live.
            let width = self
                .edge
                .max_original_operand_bits()
                .max(crate::numerical::rational_operand_bits(&sample))
                .max(crate::numerical::scratch_source_bits(
                    &[],
                    context.reference().ellipsoid(),
                ))
                .checked_mul(4)
                .and_then(|value| value.checked_add(u64::from(bits)))
                .and_then(|value| value.checked_add(64))
                .ok_or(GeoError::ArithmeticOverflow("selected point output width"))?;
            let allowance = ExactArithmeticCost::for_operation(ExactOperation::Linear, width, 8)
                .ok_or(GeoError::ArithmeticOverflow(
                    "selected point output storage",
                ))?;
            context.retain_workspace(allowance.workspace_bytes, &mut retained)?;
            progress.context_poll(context)?;
            let mut child = context.remaining_child()?;
            let point = {
                let mut nested = progress.nested(
                    context.work_items(),
                    context.current_workspace_bytes(),
                    context.workspace_peak(),
                );
                Primitive::Edge(self.edge).point(&sample, &mut child, &mut nested, physical, bits)
            };
            let point = progress.absorb_child_result(context, &child, point)?;
            let (error, _) = ExactAdmission::new(context, &mut progress).rational_retained(
                ExactOperation::RationalAdd,
                &[&point.error, &error],
                &mut retained,
                || point.error.add(&error),
            )?;
            let storage = (size_of::<LonLat>() + size_of::<Rat>()) as u64
                + point.point.longitude().allocated_bytes() as u64
                + point.point.latitude().allocated_bytes() as u64
                + error.allocated_bytes() as u64;
            if storage > retained {
                return Err(GeoError::ArithmeticOverflow(
                    "selected point output allowance",
                ));
            }
            Ok(PointSample {
                point: point.point,
                error,
                storage,
            })
        })();
        let storage = result.as_ref().map_or(0, |value| value.storage);
        context.release_workspace(retained - storage)?;
        result
    }
}
