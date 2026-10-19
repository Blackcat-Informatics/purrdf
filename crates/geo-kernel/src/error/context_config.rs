// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original worker scratch, backend and retained-storage configuration refusals.

/// The original materialized-output receipt release requirement that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutputReceiptReason {
    /// No live registration in this context matches the supplied authority.
    Unknown,
    /// A result or extra receipt still holds the registered output authority.
    LiveOwners,
}
impl purrdf_hash::error::StructuredError for OutputReceiptReason {
    purrdf_hash::error_visit! {
        Unknown => "output-receipt-unknown" {},
        LiveOwners => "output-receipt-live-owners" {},
    }
}

/// The original worker configuration and requested operation that disagree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ContextConfigError {
    /// An original materialized-output release cannot consume its receipt.
    OutputReceipt {
        /// First failed release requirement.
        reason: OutputReceiptReason,
        /// Original receipt's stated workspace.
        receipt_bytes: u64,
        /// Actual authority owners at the failed release, before receipt drop.
        live_owners: usize,
        /// Required registry and consuming-receipt owner count.
        required_owners: usize,
        /// Original number of live context registrations.
        registered_outputs: usize,
        /// Original registered carrier storage.
        output_bytes: u64,
        /// Original registration-table storage.
        receipt_storage_bytes: u64,
        /// Original context retained workspace.
        retained_workspace: u64,
        /// Original context live workspace.
        current_workspace: u64,
    },
    /// A producer's output transfer exceeds its actual live reservation.
    OutputTransfer {
        /// Original incoming output allowance.
        bytes: u64,
        /// Original materialization owner's total storage.
        previous_total: u64,
        /// Original materialization owner's output storage.
        previous_output: u64,
        /// Original checked proposed total.
        proposed_total: u64,
        /// Original checked proposed output.
        proposed_output: u64,
        /// Original context retained workspace.
        retained_workspace: u64,
        /// Original context live workspace.
        current_workspace: u64,
        /// Original checked complete reservation required by the transfer.
        required_workspace: u64,
    },
    /// An installed arena cannot be replaced through the borrowing ingress.
    ScratchInstalled {
        /// Existing arena destination capacity.
        existing_destinations: usize,
        /// Existing arena limb capacity.
        existing_limbs: usize,
        /// Incoming borrowed arena destination capacity.
        incoming_destinations: usize,
        /// Incoming borrowed arena limb capacity.
        incoming_limbs: usize,
    },
    /// A borrowed arena exceeds the configured destination ceiling.
    BorrowedDestinations {
        /// Original borrowed destination capacity.
        requested: usize,
        /// Original policy destination ceiling.
        limit: u32,
    },
    /// A numerical preparation requests more destinations than admitted.
    RequestedDestinations {
        /// Original requested destination count.
        requested: usize,
        /// Original policy destination ceiling.
        limit: u32,
        /// Original source coefficient width.
        source_bits: u64,
        /// Original requested precision.
        precision_bits: u32,
    },
    /// The selected equivalent arithmetic implementation is unavailable here.
    UnavailableBackend {
        /// Original selected implementation.
        backend: purrdf_xsd::math::FloatProductBackend,
    },
    /// Retained storage cannot exclude any still-owned allocation.
    OwnedWorkspace {
        /// Original requested retained storage.
        requested: u64,
        /// Original owned integer arena storage.
        arena: u64,
        /// Original owned geodesic coefficient storage.
        coefficients: u64,
        /// Original owned zero-conversion bound storage.
        zero_bound: u64,
        /// Original owned Taylor scratch storage.
        taylor: u64,
        /// Original materialized output storage.
        output: u64,
        /// Original materialized receipt storage.
        receipts: u64,
        /// Original checked sum of owned storage.
        required: u64,
    },
}

impl purrdf_hash::error::StructuredError for ContextConfigError {
    purrdf_hash::error_visit! {
        OutputReceipt { reason, receipt_bytes, live_owners, required_owners, registered_outputs, output_bytes, receipt_storage_bytes, retained_workspace, current_workspace } => "context-output-receipt" {
            "reason" => Record(reason), "receipt_bytes" => ByteCount(u128::from(*receipt_bytes)),
            "live_owners" => Unsigned(*live_owners as u128), "required_owners" => Unsigned(*required_owners as u128),
            "registered_outputs" => Unsigned(*registered_outputs as u128), "output_bytes" => ByteCount(u128::from(*output_bytes)),
            "receipt_storage_bytes" => ByteCount(u128::from(*receipt_storage_bytes)),
            "retained_workspace" => ByteCount(u128::from(*retained_workspace)), "current_workspace" => ByteCount(u128::from(*current_workspace))
        },
        OutputTransfer { bytes, previous_total, previous_output, proposed_total, proposed_output, retained_workspace, current_workspace, required_workspace } => "context-output-transfer" {
            "bytes" => ByteCount(u128::from(*bytes)), "previous_total" => ByteCount(u128::from(*previous_total)),
            "previous_output" => ByteCount(u128::from(*previous_output)), "proposed_total" => ByteCount(u128::from(*proposed_total)),
            "proposed_output" => ByteCount(u128::from(*proposed_output)), "retained_workspace" => ByteCount(u128::from(*retained_workspace)),
            "current_workspace" => ByteCount(u128::from(*current_workspace)), "required_workspace" => ByteCount(u128::from(*required_workspace))
        },
        ScratchInstalled { existing_destinations, existing_limbs, incoming_destinations, incoming_limbs } => "context-scratch-installed" {
            "existing_destinations" => Unsigned(*existing_destinations as u128), "existing_limbs" => Unsigned(*existing_limbs as u128),
            "incoming_destinations" => Unsigned(*incoming_destinations as u128), "incoming_limbs" => Unsigned(*incoming_limbs as u128)
        },
        BorrowedDestinations { requested, limit } => "context-borrowed-destinations" {
            "requested" => Unsigned(*requested as u128), "limit" => Unsigned(u128::from(*limit))
        },
        RequestedDestinations { requested, limit, source_bits, precision_bits } => "context-requested-destinations" {
            "requested" => Unsigned(*requested as u128), "limit" => Unsigned(u128::from(*limit)),
            "source_bits" => Unsigned(u128::from(*source_bits)), "precision_bits" => Unsigned(u128::from(*precision_bits))
        },
        UnavailableBackend { backend } => "context-unavailable-backend" { "backend" => Record(backend) },
        OwnedWorkspace { requested, arena, coefficients, zero_bound, taylor, output, receipts, required } => "context-owned-workspace" {
            "requested" => ByteCount(u128::from(*requested)), "arena" => ByteCount(u128::from(*arena)),
            "coefficients" => ByteCount(u128::from(*coefficients)), "zero_bound" => ByteCount(u128::from(*zero_bound)),
            "taylor" => ByteCount(u128::from(*taylor)), "output" => ByteCount(u128::from(*output)),
            "receipts" => ByteCount(u128::from(*receipts)), "required" => ByteCount(u128::from(*required))
        },
    }
}

impl core::fmt::Display for ContextConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::OutputReceipt { reason, .. } => match reason {
                OutputReceiptReason::Unknown => {
                    "materialized output receipt belongs to a different or released context"
                }
                OutputReceiptReason::LiveOwners => {
                    "materialized output still has a live result or receipt owner"
                }
            },
            Self::OutputTransfer { .. } => {
                "materialized output transfer exceeds its live context reservation"
            }
            Self::ScratchInstalled { .. } => "integer scratch already installed",
            Self::BorrowedDestinations { .. } => {
                "borrowed scratch exceeds admitted destination count"
            }
            Self::RequestedDestinations { .. } => {
                "scratch destination request exceeds admitted execution policy"
            }
            Self::UnavailableBackend { .. } => "unavailable binary64 product backend",
            Self::OwnedWorkspace { .. } => {
                "retained workspace cannot erase live owned numerical storage"
            }
        })
    }
}
impl std::error::Error for ContextConfigError {}
