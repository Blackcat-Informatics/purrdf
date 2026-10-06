// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Separate live materialized output from its temporary production workspace.

use crate::context::WorkProgress;
use crate::{GeoError, MaterializedOutputReceipt, MetricContext};

#[derive(Default)]
pub(crate) struct MaterializationStorage {
    total: u64,
    output: u64,
}

impl MaterializationStorage {
    pub(crate) fn admitted_transient(bytes: u64) -> Self {
        Self {
            total: bytes,
            output: 0,
        }
    }

    pub(crate) const fn output_bytes(&self) -> u64 {
        self.output
    }

    /// Transfer a completed producer's still-live reservation to this owner.
    /// The producer removes these bytes from its local counter without releasing
    /// them from the context. No allocation or second admission occurs here.
    pub(crate) fn adopt_admitted_output(
        &mut self,
        bytes: u64,
        context: &MetricContext,
    ) -> Result<(), GeoError> {
        let total = self
            .total
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "adopted materialization storage",
            ))?;
        let output = self
            .output
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("adopted materialized output"))?;
        let complete = context
            .retained_workspace_bytes()
            .checked_add(total)
            .ok_or(GeoError::ArithmeticOverflow(
                "adopted output context allowance",
            ))?;
        if complete > context.current_workspace_bytes() {
            return Err(GeoError::config(
                "materialized output transfer exceeds its live context reservation",
            ));
        }
        self.total = total;
        self.output = output;
        Ok(())
    }

    /// Verify actual final graph storage before adopting the producer's original
    /// output reservation. The returned amount is the checked transfer; the
    /// producer releases only its remaining temporary/storage allowance.
    pub(crate) fn adopt_geometry_output(
        &mut self,
        geometry: &crate::Geometry,
        available: u64,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        admit_count: impl FnOnce(u64, &MetricContext) -> Result<(), GeoError>,
    ) -> Result<u64, GeoError> {
        let owned = super::storage::owned_geometry_storage(geometry, context, progress)?;
        let bytes = owned.bytes();
        if bytes > available {
            return Err(GeoError::config(
                "union output storage exceeds its producer reservation",
            ));
        }
        admit_count(owned.coordinates(), context)?;
        self.adopt_admitted_output(bytes, context)?;
        Ok(bytes)
    }

    fn admit(
        &mut self,
        bytes: u64,
        output: bool,
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        let total = self
            .total
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("materialization storage"))?;
        let retained = if output {
            self.output
                .checked_add(bytes)
                .ok_or(GeoError::ArithmeticOverflow("materialized output storage"))?
        } else {
            self.output
        };
        context.admit_workspace(bytes)?;
        self.total = total;
        self.output = retained;
        Ok(())
    }

    pub(crate) fn admit_output(
        &mut self,
        bytes: u64,
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.admit(bytes, true, context)
    }

    pub(crate) fn admit_transient(
        &mut self,
        bytes: u64,
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.admit(bytes, false, context)
    }

    pub(crate) fn release_transient(
        &mut self,
        bytes: u64,
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.release(bytes, false, context)
    }

    /// Release a dropped original output while preserving sibling/transferred
    /// outputs and the producer's remaining temporary storage.
    pub(crate) fn release_output(
        &mut self,
        bytes: u64,
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.release(bytes, true, context)
    }

    fn release(
        &mut self,
        bytes: u64,
        output: bool,
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        let remaining_output = if output {
            self.output.checked_sub(bytes)
        } else {
            Some(self.output)
        }
        .ok_or(GeoError::ArithmeticOverflow("materialized output release"))?;
        let total = self
            .total
            .checked_sub(bytes)
            .filter(|total| *total >= remaining_output)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialization temporary release",
            ))?;
        context.release_workspace(bytes)?;
        self.total = total;
        self.output = remaining_output;
        Ok(())
    }

    /// A successful producer leaves its complete owned output charged. A refused
    /// producer drops that output before calling this and releases every charge.
    pub(crate) fn finish(
        self,
        success: bool,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Option<MaterializedOutputReceipt>, GeoError> {
        let output = if success { self.output } else { 0 };
        context.release_workspace(self.total - output)?;
        if !success {
            return Ok(None);
        }
        match context.register_materialized_output(output, progress) {
            Ok(receipt) => Ok(Some(receipt)),
            Err(error) => {
                context.release_workspace(output)?;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coord, CoordDim, Geometry, GeometryBody, Rat};

    #[test]
    fn producer_storage_transfers_once_and_survives_following_invocations() {
        let geometry = Geometry::new(
            CoordDim::Xy,
            GeometryBody::Point(Some(Coord::xy(Rat::zero(), Rat::zero()))),
        )
        .unwrap();
        let expected = size_of::<Geometry>() as u64;
        let mut context = MetricContext::wgs84().unwrap();
        let mut storage = MaterializationStorage::default();
        storage.admit_transient(32, &mut context).unwrap();
        storage.admit_output(64, &mut context).unwrap();
        context.admit_workspace(expected + 100).unwrap();
        let mut progress = WorkProgress::integer(None);
        let before = context.current_workspace_bytes();
        assert!(matches!(
            storage.adopt_geometry_output(
                &geometry,
                expected - 1,
                &mut context,
                &mut progress,
                |_, _| Ok(())
            ),
            Err(GeoError::Config(_))
        ));
        assert_eq!(storage.output_bytes(), 64);
        assert_eq!(context.current_workspace_bytes(), before);
        assert!(matches!(
            storage.adopt_geometry_output(
                &geometry,
                expected + 100,
                &mut context,
                &mut progress,
                |_, _| Err(GeoError::OutputExhausted { limit: 0 }),
            ),
            Err(GeoError::OutputExhausted { limit: 0 })
        ));
        assert_eq!(storage.output_bytes(), 64);
        assert_eq!(context.current_workspace_bytes(), before);
        assert_eq!(
            storage
                .adopt_geometry_output(
                    &geometry,
                    expected + 100,
                    &mut context,
                    &mut progress,
                    |count, _| {
                        assert_eq!(count, 1);
                        Ok(())
                    }
                )
                .unwrap(),
            expected
        );
        assert_eq!(context.current_workspace_bytes(), before);
        assert_eq!(storage.output_bytes(), expected + 64);
        // The original producer drops its temporaries, while this owner drops
        // the replaced output. The transferred graph stays covered throughout.
        context.release_workspace(100).unwrap();
        storage.release_output(64, &mut context).unwrap();
        let receipt = storage
            .finish(true, &mut context, &mut progress)
            .unwrap()
            .unwrap();
        let live = context.current_workspace_bytes();
        assert_eq!(context.retained_workspace_bytes(), live);
        context.begin_integer(1).unwrap();
        assert_eq!(context.current_workspace_bytes(), live);
        drop(geometry);
        context.release_materialized_output(receipt).unwrap();
        assert_eq!(context.current_workspace_bytes(), 0);
    }
}
