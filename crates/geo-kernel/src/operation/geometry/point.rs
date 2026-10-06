// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Symbolic dimension-zero images retain the exact source and actual mapping.

use super::{OperationChain, OperationImageCurve, OperationImageEnclosure};
use crate::context::WorkProgress;
use crate::{
    Coord, ExecutionPolicy, GeoError, GeographicReference, MetricContext, MetricWorkObserver, Rat,
};
use purrdf_hash::{Domain, hex::Digest32};
use std::sync::Arc;

const POINT_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/operation-image-point/v1");

/// One original point mapped continuously through an explicit immutable chain.
/// The private constant source curve shares the original operation evaluator;
/// this value remains a point in every prepared geometry and metric inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationImagePoint {
    image: OperationImageCurve,
    id: Digest32,
    preparation_work: u64,
}

impl OperationImagePoint {
    /// Retain a source point without treating its rounded angular output as exact.
    ///
    /// # Errors
    /// Refuses incompatible target references, missing height/epoch, invalid
    /// original domains and incomplete preparation admission.
    pub fn new(
        source: Coord,
        chain: Arc<OperationChain>,
        reference: GeographicReference,
        epoch: Option<Rat>,
        policy: ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        Self::prepare(source, chain, reference, epoch, policy, None)
    }

    /// Prepare one symbolic point with bounded work and cancellation polling.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::new`].
    pub fn new_metered(
        source: Coord,
        chain: Arc<OperationChain>,
        reference: GeographicReference,
        epoch: Option<Rat>,
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare(source, chain, reference, epoch, policy, Some(observer))
    }

    fn prepare(
        source: Coord,
        chain: Arc<OperationChain>,
        reference: GeographicReference,
        epoch: Option<Rat>,
        policy: ExecutionPolicy,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let mut context = MetricContext::new(reference.clone(), policy)?;
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.context_poll(&context)?;
        for operation in chain.operations() {
            operation
                .model()
                .validate_presence(source.z(), epoch.as_ref())?;
        }
        let image = {
            let mut nested = progress.nested(context.work_items(), 0, context.workspace_peak());
            OperationImageCurve::from_point_metered(
                source,
                chain,
                reference,
                epoch,
                policy.remaining_after(context.work_items(), context.workspace_peak())?,
                &mut nested,
            )?
        };
        context.charge_work(image.preparation_work())?;
        context.admit_workspace(image.retained_workspace_bytes())?;
        progress.context_poll(&context)?;
        let id = crate::profile::hash_fields(POINT_DOMAIN, [image.id().as_bytes().as_slice()]);
        Ok(Self {
            image,
            id,
            preparation_work: context.work_items(),
        })
    }

    /// Exact source, operation, epoch and target identity; admission is excluded.
    #[must_use]
    pub const fn id(&self) -> Digest32 {
        self.id
    }

    /// Actual target reference of this geometric point.
    #[must_use]
    pub const fn reference(&self) -> &GeographicReference {
        self.image.reference()
    }

    /// Original source ordinates, including caller-supplied Z and M.
    #[must_use]
    pub fn source(&self) -> &Coord {
        self.image.source_endpoints().0
    }

    /// Borrow the exact constant image law for shared original-curve topology.
    pub(crate) const fn original_curve(&self) -> &OperationImageCurve {
        &self.image
    }

    /// Complete logical preparation work, excluded from content identity.
    #[must_use]
    pub const fn preparation_work_items(&self) -> u64 {
        self.preparation_work
    }

    /// Conservative retained original source and mapping storage allowance.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.image.retained_workspace_bytes()
    }

    /// Materialize the point on the scalar chain's fixed final output grid.
    /// The returned axes are the actual target declaration. This export never
    /// replaces the original symbolic point in metric or topology inventories.
    ///
    /// # Errors
    /// Uses the chain's domain, rounding, residual and resource refusal contract.
    pub fn materialize(
        &self,
        context: &mut MetricContext,
    ) -> Result<super::super::TransformResult, GeoError> {
        self.materialize_inner(context, None)
    }

    /// Export the scalar point with bounded governor polling.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::materialize`].
    pub fn materialize_metered(
        &self,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<super::super::TransformResult, GeoError> {
        self.materialize_inner(context, Some(observer))
    }

    fn materialize_inner(
        &self,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<super::super::TransformResult, GeoError> {
        if context.reference() != self.reference() {
            return Err(GeoError::config(
                "symbolic point/context target reference mismatch",
            ));
        }
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.context_poll(context)?;
        context.admit_workspace(self.retained_workspace_bytes())?;
        let result = self.image.materialize_source_point(context, &mut progress);
        context.release_workspace(self.retained_workspace_bytes())?;
        result
    }

    /// Enclose the original point's unrounded target image at explicit precision.
    ///
    /// # Errors
    /// Refuses unresolved domains or insufficient precision/work/memory.
    pub fn enclosure(
        &self,
        precision_bits: u32,
        context: &mut MetricContext,
    ) -> Result<OperationImageEnclosure, GeoError> {
        self.image
            .enclosure(&Rat::zero(), &Rat::zero(), precision_bits, context)
    }

    /// Enclose the unrounded image with bounded governor callbacks.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::enclosure`].
    pub fn enclosure_metered(
        &self,
        precision_bits: u32,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<OperationImageEnclosure, GeoError> {
        self.image.enclosure_metered(
            &Rat::zero(),
            &Rat::zero(),
            precision_bits,
            context,
            observer,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::{
        CoordinateOperation, CoordinateUnit, OperationModel, OperationReference,
    };

    fn mapping(reference: &GeographicReference) -> Arc<OperationChain> {
        Arc::new(
            OperationChain::compile(vec![
                CoordinateOperation::compile(
                    OperationReference {
                        realization: Digest32::new([9; 32]),
                        unit: CoordinateUnit::Metres,
                        swapped_axes: false,
                    },
                    OperationReference {
                        realization: reference.id().digest(),
                        unit: CoordinateUnit::Degrees,
                        swapped_axes: false,
                    },
                    OperationModel::MercatorToGeographic {
                        radius: Rat::from_i64(6_378_137),
                        eccentricity_squared: Rat::zero(),
                        square_domain: true,
                    },
                )
                .unwrap(),
            ])
            .unwrap(),
        )
    }

    #[test]
    fn original_point_image_is_unrounded_and_admission_independent() {
        let reference = GeographicReference::wgs84();
        let source = Coord::xy(Rat::one(), Rat::zero());
        let mut one_point_limits = *ExecutionPolicy::geometry().limits();
        one_point_limits.max_output_elements = 1;
        let point = OperationImagePoint::new(
            source.clone(),
            mapping(&reference),
            reference.clone(),
            None,
            ExecutionPolicy::new(one_point_limits).unwrap(),
        )
        .unwrap();
        assert_eq!(point.source(), &source);
        let mut context = MetricContext::wgs84().unwrap();
        let image = point.enclosure(128, &mut context).unwrap();
        assert!(image.longitude.lower() < image.longitude.upper());
        assert!(image.latitude.is_exact_zero());
        let mut limits = *ExecutionPolicy::geometry().limits();
        limits.max_work_items *= 2;
        let adequate = OperationImagePoint::new(
            source,
            mapping(&reference),
            reference,
            None,
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert_eq!(point.id(), adequate.id());
    }

    #[test]
    fn preparation_cancellation_returns_no_symbolic_point() {
        struct Stop;
        impl MetricWorkObserver for Stop {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        let reference = GeographicReference::wgs84();
        assert!(matches!(
            OperationImagePoint::new_metered(
                Coord::xy(Rat::one(), Rat::zero()),
                mapping(&reference),
                reference,
                None,
                ExecutionPolicy::geometry(),
                &mut Stop,
            ),
            Err(GeoError::Cancelled)
        ));
    }
}
