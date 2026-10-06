// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed ownership of materialized carriers across numerical invocations.

use super::{MetricContext, WorkProgress};
use crate::GeoError;
use std::sync::Arc;

/// Unforgeable receipt for one materialized carrier's live context allowance.
/// Obtain it from the result before dropping that result, then release it through
/// its original context. Every result or receipt clone must be dropped first;
/// a receipt from another context or a repeated release is refused.
#[derive(Clone, Debug)]
pub struct MaterializedOutputReceipt {
    token: Arc<()>,
    bytes: u64,
}
impl MaterializedOutputReceipt {
    /// Conservative carrier/result/token bytes registered in the context baseline.
    #[must_use]
    pub const fn workspace_bytes(&self) -> u64 {
        self.bytes
    }
}

impl MetricContext {
    pub(crate) fn register_materialized_output(
        &mut self,
        bytes: u64,
        progress: &mut WorkProgress<'_>,
    ) -> Result<MaterializedOutputReceipt, GeoError> {
        let bytes = bytes
            .checked_add((2 * size_of::<usize>()) as u64)
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
        let grows = next > self.materialized_outputs.capacity();
        let new_table = if grows {
            u64::try_from(next)
                .ok()
                .and_then(|count| count.checked_mul(size_of::<MaterializedOutputReceipt>() as u64))
                .ok_or(GeoError::ArithmeticOverflow("materialized receipt table"))?
        } else {
            0
        };
        let additional = (2 * size_of::<usize>() as u64)
            .checked_add(new_table)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized receipt allocation",
            ))?;
        let old_table = if grows {
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
        self.charge_work(1)?;
        self.admit_workspace(additional)?;
        let admission = progress.context_poll(self).and_then(|()| {
            self.materialized_outputs
                .try_reserve_exact(1)
                .map_err(|_| GeoError::MemoryExhausted {
                    limit: self.policy.limits().max_workspace_bytes,
                })
        });
        if let Err(error) = admission {
            self.release_workspace(additional)?;
            return Err(error);
        }
        let receipt = MaterializedOutputReceipt {
            token: Arc::new(()),
            bytes,
        };
        self.materialized_outputs.push(receipt.clone());
        self.materialized_output_bytes = outputs;
        self.retained_workspace_bytes = baseline;
        if grows {
            self.materialized_receipt_storage_bytes = new_table;
            self.workspace_bytes =
                self.workspace_bytes
                    .checked_sub(old_table)
                    .ok_or(GeoError::ArithmeticOverflow(
                        "replaced materialized receipt storage",
                    ))?;
        }
        Ok(receipt)
    }

    /// Release a materialized carrier after its last result owner is dropped.
    /// Its registered allowance survives every intervening numerical `begin`.
    /// Immutable numerical preparation and the receipt table remain admitted.
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
            .position(|entry| Arc::ptr_eq(&entry.token, &receipt.token))
            .ok_or_else(|| {
                GeoError::config(
                    "materialized output receipt belongs to a different or released context",
                )
            })?;
        if Arc::strong_count(&receipt.token) != 2 {
            return Err(GeoError::config(
                "materialized output still has a live result or receipt owner",
            ));
        }
        let outputs = self
            .materialized_output_bytes
            .checked_sub(receipt.bytes)
            .ok_or(GeoError::ArithmeticOverflow("materialized output release"))?;
        let retained = self
            .retained_workspace_bytes
            .checked_sub(receipt.bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized baseline release",
            ))?;
        let current = self
            .workspace_bytes
            .checked_sub(receipt.bytes)
            .filter(|current| *current >= retained)
            .ok_or(GeoError::ArithmeticOverflow(
                "materialized current storage release",
            ))?;
        self.materialized_outputs.swap_remove(index);
        self.materialized_output_bytes = outputs;
        self.retained_workspace_bytes = retained;
        self.workspace_bytes = current;
        drop(receipt);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_receipts_track_identity_clones_and_grown_registry_storage() {
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
        assert!(matches!(
            other.release_materialized_output(receipts[0].clone()),
            Err(GeoError::Config(_))
        ));
        assert_eq!(context.retained_workspace_bytes(), retained);
        let clone = receipts[0].clone();
        assert!(matches!(
            context.release_materialized_output(clone),
            Err(GeoError::Config(_))
        ));
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
    }
}
