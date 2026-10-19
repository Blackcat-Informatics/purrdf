// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed ownership of materialized carriers across numerical invocations.

use super::{MetricContext, Retained, WorkProgress};
use crate::GeoError;

/// Unforgeable receipt for one materialized carrier's live context allowance.
/// Obtain it from the result before dropping that result, then release it through
/// its original context. Every result or receipt clone must be dropped first;
/// a receipt from another context or a repeated release is refused.
#[derive(Clone, Debug)]
pub struct MaterializedOutputReceipt {
    token: Retained<()>,
    bytes: u64,
}
impl purrdf_hash::error::StructuredError for MaterializedOutputReceipt {
    purrdf_hash::error_visit! {
        @record self { token, bytes } => "materialized-output-receipt" {
            // The token is resource authority, not semantic data. Inspection
            // neither exposes its address nor clones its ownership capability.
            "bytes" => ByteCount(u128::from(self.bytes))
        }
    }
}
impl MaterializedOutputReceipt {
    /// Carrier/result allowance and the actual retained token allocation
    /// registered in the original context baseline.
    #[must_use]
    pub const fn workspace_bytes(&self) -> u64 {
        self.bytes
    }
}

impl MetricContext {
    fn output_receipt_failure(
        &self,
        receipt: &MaterializedOutputReceipt,
        reason: crate::OutputReceiptReason,
        live_owners: usize,
    ) -> GeoError {
        GeoError::retained_failure(
            crate::ContextConfigError::OutputReceipt {
                reason,
                receipt_bytes: receipt.bytes,
                live_owners,
                required_owners: 2,
                registered_outputs: self.materialized_outputs.len(),
                output_bytes: self.materialized_output_bytes,
                receipt_storage_bytes: self.materialized_receipt_storage_bytes,
                retained_workspace: self.retained_workspace_bytes,
                current_workspace: self.workspace_bytes,
            },
            GeoError::ContextConfiguration,
            None,
        )
    }
    pub(crate) fn register_materialized_output(
        &mut self,
        bytes: u64,
        progress: &mut WorkProgress<'_>,
    ) -> Result<MaterializedOutputReceipt, GeoError> {
        let token_bytes = Retained::<()>::allocation_layout()
            .size()
            .checked_add(progress.retained_owner_layout().size())
            .and_then(|size| u64::try_from(size).ok())
            .ok_or(GeoError::ArithmeticOverflow("materialized output token"))?;
        let bytes = bytes
            .checked_add(token_bytes)
            .ok_or(GeoError::ArithmeticOverflow("materialized output token"))?;
        let outputs = self
            .materialized_output_bytes
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("live materialized outputs"))?;
        let next = self
            .materialized_outputs
            .len()
            .checked_add(1)
            .ok_or(GeoError::ArithmeticOverflow("materialized receipt count"))?;
        let replacement = self
            .materialized_outputs
            .replacement_bytes(next, progress)?;
        let new_table = u64::try_from(replacement.unwrap_or(0))
            .map_err(|_| GeoError::ArithmeticOverflow("materialized receipt table"))?;
        let additional = token_bytes
            .checked_add(new_table)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized receipt allocation",
            ))?;
        let old_table = if replacement.is_some() {
            self.materialized_receipt_storage_bytes
        } else {
            0
        };
        let baseline = self
            .retained_workspace_bytes
            .checked_add(bytes)
            .and_then(|value| value.checked_add(new_table))
            .and_then(|value| value.checked_sub(old_table))
            .ok_or(GeoError::ArithmeticOverflow("materialized output baseline"))?;
        let original_workspace = self.workspace_bytes;
        let workspace = original_workspace
            .checked_add(additional)
            .and_then(|value| value.checked_sub(old_table))
            .filter(|value| *value >= baseline)
            .ok_or(GeoError::ArithmeticOverflow("materialized output storage"))?;
        self.charge_work(1)?;
        self.admit_workspace(additional)?;
        // The working registry stays under its original owner while both
        // replacement allocations are born. No receipt is published until
        // the token and the destination slot exist.
        let mut registry = core::mem::take(&mut self.materialized_outputs);
        let admission = progress.context_poll(self).and_then(|()| {
            let token = Retained::produce(progress, |_| Ok(()), |(), _| Ok(()))?;
            registry.reserve(next, progress)?;
            let receipt = MaterializedOutputReceipt { token, bytes };
            registry.push_reserved(receipt.clone());
            Ok(receipt)
        });
        self.materialized_outputs = registry;
        let receipt = match admission {
            Ok(receipt) => receipt,
            Err(error) => {
                // Refused token/replacement payloads have died. The original
                // registry is unchanged; restore only this invocation's fee
                // without replacing the first physical refusal.
                self.workspace_bytes = original_workspace;
                return Err(error);
            }
        };
        self.materialized_output_bytes = outputs;
        self.retained_workspace_bytes = baseline;
        self.workspace_bytes = workspace;
        if replacement.is_some() {
            self.materialized_receipt_storage_bytes = new_table;
        }
        Ok(receipt)
    }

    /// Release a materialized carrier after its last result owner is dropped.
    /// Its registered allowance survives every intervening numerical `begin`.
    /// Immutable numerical preparation remains admitted. The receipt table is
    /// destroyed after its last registered output is released.
    ///
    /// # Errors
    /// Refuses a wrong context, a repeated release, live result/receipt clones,
    /// or accounting inconsistency without changing the registered allowance.
    pub fn release_materialized_output(
        &mut self,
        receipt: MaterializedOutputReceipt,
    ) -> Result<(), GeoError> {
        let index = self
            .materialized_outputs
            .iter()
            .position(|entry| Retained::ptr_eq(&entry.token, &receipt.token))
            .ok_or_else(|| {
                self.output_receipt_failure(
                    &receipt,
                    crate::OutputReceiptReason::Unknown,
                    receipt.token.strong_count(),
                )
            })?;
        let live_owners = receipt.token.strong_count();
        if live_owners != 2 {
            return Err(self.output_receipt_failure(
                &receipt,
                crate::OutputReceiptReason::LiveOwners,
                live_owners,
            ));
        }
        let outputs = self
            .materialized_output_bytes
            .checked_sub(receipt.bytes)
            .ok_or(GeoError::ArithmeticOverflow("materialized output release"))?;
        let table_bytes = if self.materialized_outputs.len() == 1 {
            self.materialized_receipt_storage_bytes
        } else {
            0
        };
        let released_bytes =
            receipt
                .bytes
                .checked_add(table_bytes)
                .ok_or(GeoError::ArithmeticOverflow(
                    "materialized registry release",
                ))?;
        let retained = self
            .retained_workspace_bytes
            .checked_sub(released_bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized baseline release",
            ))?;
        let current = self
            .workspace_bytes
            .checked_sub(released_bytes)
            .filter(|current| *current >= retained)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized current storage release",
            ))?;
        // Drop both original references before refunding their control or
        // carrier allowance. Other result owners were excluded above.
        drop(self.materialized_outputs.swap_remove(index));
        drop(receipt);
        if self.materialized_outputs.is_empty() {
            // The now-empty buffer dies before its original control allowance
            // is refunded; no future invocation retains hypothetical capacity.
            drop(core::mem::take(&mut self.materialized_outputs));
            self.materialized_receipt_storage_bytes = 0;
        }
        self.materialized_output_bytes = outputs;
        self.retained_workspace_bytes = retained;
        self.workspace_bytes = current;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_receipts_track_identity_clones_and_grown_registry_storage() {
        use crate::{ContextConfigError, OutputReceiptReason};
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = WorkProgress::new(None);
        let mut receipts = Vec::new();
        for _ in 0..6 {
            context.admit_workspace(256).unwrap();
            receipts.push(
                context
                    .register_materialized_output(256, &mut progress)
                    .unwrap(),
            );
        }
        assert!(
            context.materialized_receipt_storage_bytes
                >= 6 * size_of::<MaterializedOutputReceipt>() as u64
        );
        let retained = context.retained_workspace_bytes();
        context.begin(1).unwrap();
        assert_eq!(context.retained_workspace_bytes(), retained);
        let mut other = MetricContext::wgs84().unwrap();
        let receipt_bytes = receipts[0].workspace_bytes();
        let unknown = other
            .release_materialized_output(receipts[0].clone())
            .unwrap_err();
        assert!(
            matches!(unknown.original(), GeoError::ContextConfiguration(cause)
            if matches!(&**cause, ContextConfigError::OutputReceipt { reason: OutputReceiptReason::Unknown, receipt_bytes: bytes, live_owners: 3, required_owners: 2, registered_outputs: 0, .. } if *bytes == receipt_bytes))
        );
        assert_eq!(context.retained_workspace_bytes(), retained);
        let clone = receipts[0].clone();
        let live = context.release_materialized_output(clone).unwrap_err();
        assert!(
            matches!(live.original(), GeoError::ContextConfiguration(cause)
            if matches!(&**cause, ContextConfigError::OutputReceipt { reason: OutputReceiptReason::LiveOwners, receipt_bytes: bytes, live_owners: 3, required_owners: 2, registered_outputs: 6, retained_workspace, .. } if *bytes == receipt_bytes && *retained_workspace == retained))
        );
        let fields = Capture::read(&live).child("cause");
        assert!(
            matches!(fields.field("receipt_bytes"), ErrorValue::ByteCount(bytes) if bytes == u128::from(receipt_bytes))
        );
        assert!(matches!(
            fields.field("live_owners"),
            ErrorValue::Unsigned(3)
        ));
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let last = live.clone();
        assert_eq!(window.close().allocations, 0);
        for receipt in receipts {
            context.release_materialized_output(receipt).unwrap();
        }
        assert_eq!(context.materialized_output_bytes, 0);
        assert_eq!(
            context.retained_workspace_bytes(),
            context.materialized_receipt_storage_bytes
        );
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            context.policy.limits().max_workspace_bytes
        );
        // Keeping all refusal records alive did not retain receipt authority.
        assert_eq!(
            Capture::read(&unknown).child("cause").child("reason").code,
            "output-receipt-unknown"
        );
        assert_eq!(
            Capture::read(&last).child("cause").child("reason").code,
            "output-receipt-live-owners"
        );
    }
}
