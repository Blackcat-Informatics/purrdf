// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Separate live materialized output from its temporary production workspace.

use crate::context::{Produced, RetainedBuilder, WorkProgress};
use crate::{GeoError, MaterializedOutputReceipt, MetricContext};
use purrdf_lex::allocation::SharedOwned;

#[derive(Default)]
pub(crate) struct MaterializationStorage {
    total: u64,
    output: u64,
    originals: RetainedBuilder<SharedOwned<()>>,
}

impl MaterializationStorage {
    /// The enclosing phase's original policy counter. Working containers retain
    /// their physical accounts independently and refund only after destruction.
    pub(crate) fn working_counter(&mut self) -> &mut u64 {
        &mut self.total
    }

    pub(crate) fn retain_native<T: crate::context::NativeDetached>(
        &mut self,
        value: T,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<T, GeoError> {
        let previous = self.total;
        let value = crate::context::persistent_numeric(value, context, progress, &mut self.total)?;
        self.output =
            self.output
                .checked_add(self.total.checked_sub(previous).ok_or(
                    GeoError::ArithmeticOverflow("materialized native scalar receipt"),
                )?)
                .ok_or(GeoError::ArithmeticOverflow(
                    "materialized native scalar output",
                ))?;
        Ok(value)
    }

    fn reserve_original(
        &mut self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        let previous = self.total;
        let required = self
            .originals
            .len()
            .checked_add(1)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized original owner count",
            ))?;
        self.originals
            .reserve_admitted(required, context, progress, &mut self.total)?;
        let metadata = self
            .total
            .checked_sub(previous)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized owner replacement",
            ))?;
        self.output = self
            .output
            .checked_add(metadata)
            .ok_or(GeoError::ArithmeticOverflow("materialized owner metadata"))?;
        Ok(())
    }

    /// Consume one already admitted working array into the exact public carrier
    /// layout. Cells move without a clone; the old buffer and control die before
    /// their receipt is refunded. The new account slot exists before extraction.
    pub(crate) fn output_array<T>(
        &mut self,
        values: RetainedBuilder<T>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Vec<T>, GeoError> {
        if values.is_empty() {
            values.release_admitted(context, &mut self.total)?;
            return Ok(Vec::new());
        }
        let previous = u64::try_from(values.admitted_bytes())
            .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?;
        self.reserve_original(context, progress)?;
        let bytes = core::alloc::Layout::array::<T>(values.len())
            .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?
            .size()
            .checked_add(progress.retained_owner_layout().size())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized native array layout",
            ))?;
        context.retain_workspace(bytes, &mut self.total)?;
        let result = progress.context_poll(context).and_then(|()| {
            Produced::produce(
                progress,
                core::alloc::Layout::new::<()>(),
                |memory| {
                    let mut output = Vec::new();
                    memory
                        .reserve(&mut output, values.len())
                        .map_err(GeoError::Storage)?;
                    for value in values {
                        output.push(value);
                    }
                    Ok(output)
                },
                |_, _| Ok(()),
            )
        });
        let released = context.release_retained_workspace(previous, &mut self.total);
        let produced = match progress.settle_result(result, released) {
            Ok(produced) => produced,
            Err(error) => {
                let _ = context.release_retained_workspace(bytes, &mut self.total);
                return Err(error);
            }
        };
        if u64::try_from(produced.admitted_bytes()).ok() != Some(bytes) {
            drop(produced);
            let _ = context.release_retained_workspace(bytes, &mut self.total);
            return Err(GeoError::ArithmeticOverflow(
                "materialized native array certificate",
            ));
        }
        self.output = self
            .output
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized native array receipt",
            ))?;
        let (values, _, original) = produced.into_parts();
        self.originals.push_reserved(original);
        Ok(values)
    }

    pub(crate) fn admitted_transient(bytes: u64) -> Self {
        Self {
            total: bytes,
            output: 0,
            originals: RetainedBuilder::default(),
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
            return Err(GeoError::retained_failure(
                crate::ContextConfigError::OutputTransfer {
                    bytes,
                    previous_total: self.total,
                    previous_output: self.output,
                    proposed_total: total,
                    proposed_output: output,
                    retained_workspace: context.retained_workspace_bytes(),
                    current_workspace: context.current_workspace_bytes(),
                    required_workspace: complete,
                },
                GeoError::ContextConfiguration,
                None,
            ));
        }
        self.total = total;
        self.output = output;
        Ok(())
    }

    /// Verify a resident fixture's final graph storage before adopting its
    /// output reservation. The returned amount is the checked transfer; the
    /// producer releases only its remaining temporary/storage allowance.
    #[cfg(test)]
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

    /// Move a native geometry only after its original account has an admitted
    /// slot in the enclosing output owner. The supplied allowance was retained
    /// by its actual producers before birth; no storage census is substituted.
    pub(crate) fn adopt_produced_geometry_output(
        &mut self,
        geometry: Produced<crate::Geometry>,
        available: u64,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        admit_count: impl FnOnce(u64, &MetricContext) -> Result<(), GeoError>,
    ) -> Result<(crate::Geometry, u64), GeoError> {
        let mut count = 0_u64;
        let counted =
            crate::geom::visit_coords_with_progress(&geometry, progress, |_, progress| {
                context.charge_work(1)?;
                progress.context_poll(context)?;
                count = count.checked_add(1).ok_or(GeoError::ArithmeticOverflow(
                    "materialized output coordinates",
                ))?;
                Ok(())
            });
        counted?;
        admit_count(count, context)?;
        self.reserve_original(context, progress)?;
        self.adopt_admitted_output(available, context)?;
        let (geometry, _, original) = geometry.into_parts();
        self.originals.push_reserved(original);
        Ok((geometry, available))
    }

    pub(crate) fn publish_literal(
        &mut self,
        literal: crate::GeometryLiteral,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<crate::context::Retained<MaterializedLiteral>, GeoError> {
        let value = MaterializedLiteral {
            literal,
            _originals: crate::context::RetainedValues::from_admitted(core::mem::take(
                &mut self.originals,
            )),
        };
        let previous = self.total;
        let produced =
            crate::context::Retained::new_admitted(value, context, progress, &mut self.total)?;
        self.output = self
            .output
            .checked_add(
                self.total
                    .checked_sub(previous)
                    .ok_or(GeoError::ArithmeticOverflow("materialized literal control"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("materialized output control"))?;
        Ok(produced)
    }

    /// Publish the original current native geometry-buffer account together
    /// with the independently retained arrays and scalar owners. The complete
    /// buffer allowance has already been recorded at each original birth.
    pub(crate) fn publish_original_literal(
        &mut self,
        literal: crate::GeometryLiteral,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<crate::context::Retained<MaterializedLiteral>, GeoError> {
        let controls = crate::context::Retained::<MaterializedLiteral>::required_bytes(progress)?;
        context.retain_workspace(controls, &mut self.total)?;
        self.output = self
            .output
            .checked_add(controls)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized original controls",
            ))?;
        crate::context::Retained::publish_original(
            MaterializedLiteral {
                literal,
                _originals: crate::context::RetainedValues::from_admitted(core::mem::take(
                    &mut self.originals,
                )),
            },
            progress,
        )
    }

    /// Settle the complete output while it still owns every original account.
    /// Any failed receipt/control publication destroys the payload before its
    /// policy allowance is refunded, retaining the selected first failure.
    pub(crate) fn finish_result(
        self,
        result: Result<crate::context::Retained<MaterializedLiteral>, GeoError>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<
        (
            crate::context::Retained<MaterializedLiteral>,
            MaterializedOutputReceipt,
        ),
        GeoError,
    > {
        let Self {
            total,
            output,
            originals,
        } = self;
        let value = match progress.settle_result(result, Ok(())) {
            Ok(value) => value,
            Err(error) => {
                drop(originals);
                return progress.settle_result(Err(error), context.release_workspace(total));
            }
        };
        if !originals.is_empty() {
            drop(value);
            drop(originals);
            return progress.settle_result(
                Err(GeoError::ArithmeticOverflow(
                    "unpublished materialized original owner",
                )),
                context.release_workspace(total),
            );
        }
        drop(originals);
        let Some(transient) = total.checked_sub(output) else {
            drop(value);
            return progress.settle_result(
                Err(GeoError::ArithmeticOverflow("materialized final allowance")),
                context.release_workspace(total),
            );
        };
        if let Err(error) = context.release_workspace(transient) {
            drop(value);
            return progress.settle_result(Err(error), context.release_workspace(total));
        }
        match context.register_materialized_output(output, progress) {
            Ok(receipt) => Ok((value, receipt)),
            Err(error) => {
                drop(value);
                progress.settle_result(Err(error), context.release_workspace(output))
            }
        }
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

    #[cfg(test)]
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
    #[cfg(test)]
    pub(crate) fn finish(
        self,
        success: bool,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Option<MaterializedOutputReceipt>, GeoError> {
        let Self {
            total,
            output,
            originals,
        } = self;
        let output = if success { output } else { 0 };
        drop(originals);
        context.release_workspace(total - output)?;
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

#[derive(Debug)]
pub(crate) struct MaterializedLiteral {
    pub(crate) literal: crate::GeometryLiteral,
    // The native raw geometry buffers die before their original accounts.
    _originals: crate::context::RetainedValues<SharedOwned<()>>,
}

impl PartialEq for MaterializedLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.literal == other.literal
    }
}
impl Eq for MaterializedLiteral {}

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
        let previous_total = storage.total;
        let previous_output = storage.output;
        let incoming = before + 1;
        let error = storage
            .adopt_admitted_output(incoming, &context)
            .unwrap_err();
        assert!(
            matches!(error.original(), GeoError::ContextConfiguration(cause)
            if **cause == crate::ContextConfigError::OutputTransfer {
                bytes: incoming, previous_total, previous_output,
                proposed_total: previous_total + incoming, proposed_output: previous_output + incoming,
                retained_workspace: context.retained_workspace_bytes(), current_workspace: before,
                required_workspace: context.retained_workspace_bytes() + previous_total + incoming,
            })
        );
        assert_eq!(
            (
                storage.total,
                storage.output,
                context.current_workspace_bytes()
            ),
            (previous_total, previous_output, before)
        );
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
