// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original prepared-source receipts and context admission mismatches.

/// The first absent part of a prepared-source admission baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreparationBaselineReason {
    /// The context has not admitted the required preparation work.
    Work,
    /// The context has not retained the required source storage.
    Workspace,
}
impl purrdf_hash::error::StructuredError for PreparationBaselineReason {
    purrdf_hash::error_visit! {
        Work => "preparation-baseline-work" {},
        Workspace => "preparation-baseline-workspace" {},
    }
}

/// The original operation consuming a mismatched source receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreparationReceiptConsumer {
    /// A geographic index query.
    GeographicIndex,
    /// A relation over prepared geographic sources.
    GeographicRelation,
    /// Materialization of a source-bound buffer.
    BufferMaterialization,
    /// Membership in a source-bound offset.
    OffsetMembership,
}
impl purrdf_hash::error::StructuredError for PreparationReceiptConsumer {
    purrdf_hash::error_visit! {
        GeographicIndex => "preparation-receipt-geographic-index" {},
        GeographicRelation => "preparation-receipt-geographic-relation" {},
        BufferMaterialization => "preparation-receipt-buffer-materialization" {},
        OffsetMembership => "preparation-receipt-offset-membership" {},
    }
}

/// The original receipts and resource quantities at a source-admission refusal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SourcePreparationError {
    /// The actual operation-cell Jordan witness disagrees with the requested traversal.
    JordanCertificate {
        /// Original certified chart availability.
        physical_chart: bool,
        /// Original certified orientation sign.
        orientation: i8,
        /// Original requested traversal reversal.
        reversed: bool,
        /// Original curve edge count.
        edge_count: usize,
    },
    /// An individual source is absent from the context admission baseline.
    Baseline {
        /// Original source receipt.
        receipt: crate::PreparedSourceReceipt,
        /// First requirement that failed.
        reason: PreparationBaselineReason,
        /// Original context preparation work.
        actual_work: u64,
        /// Original context retained workspace.
        actual_workspace: u64,
    },
    /// A receipt does not describe the immutable source being evaluated.
    Receipt {
        /// Original consuming operation.
        consumer: PreparationReceiptConsumer,
        /// Original immutable source receipt.
        expected: crate::PreparedSourceReceipt,
        /// Original supplied receipt.
        actual: crate::PreparedSourceReceipt,
    },
    /// The complete pair of distinct sources is absent from the relation baseline.
    RelationAdmission {
        /// Original left receipt.
        first: crate::PreparedSourceReceipt,
        /// Original right receipt.
        second: crate::PreparedSourceReceipt,
        /// First requirement that failed.
        reason: PreparationBaselineReason,
        /// Original complete pair's required work.
        required_work: u64,
        /// Original complete pair's required workspace.
        required_workspace: u64,
        /// Original context preparation work.
        actual_work: u64,
        /// Original context retained workspace.
        actual_workspace: u64,
    },
    /// A child baseline exceeds its original parent or child preparation.
    ChildBaseline {
        /// False while creating a child, true while absorbing its completed work.
        absorbing: bool,
        /// First requirement that failed in the original comparison order.
        reason: PreparationBaselineReason,
        /// Original requested baseline work.
        requested_work: u64,
        /// Original requested baseline storage.
        requested_workspace: u64,
        /// Original available preparation work.
        available_work: u64,
        /// Original available retained storage.
        available_workspace: u64,
    },
    /// A source-bound offset constructor is absent from the original baseline.
    OffsetBaseline {
        /// Original prepared source receipt.
        source: crate::PreparedSourceReceipt,
        /// First absent baseline requirement.
        reason: PreparationBaselineReason,
        /// Original source plus offset preparation work.
        required_work: u64,
        /// Original offset retained storage.
        required_workspace: u64,
        /// Original context preparation work.
        actual_work: u64,
        /// Original context retained storage.
        actual_workspace: u64,
    },
    /// A cumulative budget was installed on a context with another policy.
    BudgetPolicy {
        /// Original cumulative preparation budget.
        budget: crate::PreparationBudget,
        /// Original target context policy.
        actual: crate::ExecutionPolicy,
    },
    /// An offset source is absent from the cumulative preparation budget.
    OffsetAdmission {
        /// Original prepared source receipt.
        source: crate::PreparedSourceReceipt,
        /// Original cumulative preparation budget.
        budget: crate::PreparationBudget,
        /// First missing source admission.
        reason: PreparationBaselineReason,
    },
    /// The completed offset cannot account for its unchanged original source.
    OffsetConstruction {
        /// Original prepared source receipt.
        source: crate::PreparedSourceReceipt,
        /// Original cumulative preparation budget.
        budget: crate::PreparationBudget,
        /// Completed offset's preparation work.
        result_work: u64,
        /// Completed offset's retained workspace.
        result_workspace: u64,
    },
}
impl purrdf_hash::error::StructuredError for SourcePreparationError {
    purrdf_hash::error_visit! {
        JordanCertificate { physical_chart, orientation, reversed, edge_count } => "source-preparation-jordan-certificate" {
            "physical_chart" => Boolean(*physical_chart), "orientation" => Signed(i128::from(*orientation)),
            "reversed" => Boolean(*reversed), "edge_count" => Unsigned(*edge_count as u128)
        },
        Baseline { receipt, reason, actual_work, actual_workspace } => "source-preparation-baseline" {
            "receipt" => Record(receipt), "reason" => Record(reason),
            "actual_work" => Unsigned(u128::from(*actual_work)), "actual_workspace" => ByteCount(u128::from(*actual_workspace))
        },
        Receipt { consumer, expected, actual } => "source-preparation-receipt" {
            "consumer" => Record(consumer), "expected" => Record(expected), "actual" => Record(actual)
        },
        RelationAdmission { first, second, reason, required_work, required_workspace, actual_work, actual_workspace } => "source-preparation-relation-admission" {
            "first" => Record(first), "second" => Record(second), "reason" => Record(reason),
            "required_work" => Unsigned(u128::from(*required_work)), "required_workspace" => ByteCount(u128::from(*required_workspace)),
            "actual_work" => Unsigned(u128::from(*actual_work)), "actual_workspace" => ByteCount(u128::from(*actual_workspace))
        },
        ChildBaseline { absorbing, reason, requested_work, requested_workspace, available_work, available_workspace } => "source-preparation-child-baseline" {
            "absorbing" => Boolean(*absorbing), "reason" => Record(reason),
            "requested_work" => Unsigned(u128::from(*requested_work)), "requested_workspace" => ByteCount(u128::from(*requested_workspace)),
            "available_work" => Unsigned(u128::from(*available_work)), "available_workspace" => ByteCount(u128::from(*available_workspace))
        },
        OffsetBaseline { source, reason, required_work, required_workspace, actual_work, actual_workspace } => "source-preparation-offset-baseline" {
            "source" => Record(source), "reason" => Record(reason),
            "required_work" => Unsigned(u128::from(*required_work)), "required_workspace" => ByteCount(u128::from(*required_workspace)),
            "actual_work" => Unsigned(u128::from(*actual_work)), "actual_workspace" => ByteCount(u128::from(*actual_workspace))
        },
        BudgetPolicy { budget, actual } => "source-preparation-budget-policy" { "budget" => Record(budget), "actual" => Record(actual) },
        OffsetAdmission { source, budget, reason } => "source-preparation-offset-admission" {
            "source" => Record(source), "budget" => Record(budget), "reason" => Record(reason)
        },
        OffsetConstruction { source, budget, result_work, result_workspace } => "source-preparation-offset-construction" {
            "source" => Record(source), "budget" => Record(budget),
            "result_work" => Unsigned(u128::from(*result_work)), "result_workspace" => ByteCount(u128::from(*result_workspace))
        },
    }
}
impl core::fmt::Display for SourcePreparationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::JordanCertificate { .. } => "operation cell Jordan certificate mismatch",
            Self::Baseline { .. } => {
                "prepared source is absent from the context admission baseline"
            }
            Self::Receipt { consumer, .. } => match consumer {
                PreparationReceiptConsumer::GeographicIndex => {
                    "geographic index source receipt does not match its immutable preparation"
                }
                PreparationReceiptConsumer::GeographicRelation => {
                    "geographic relation receipt names a different source"
                }
                PreparationReceiptConsumer::BufferMaterialization => {
                    "buffer receipt names a different original source"
                }
                PreparationReceiptConsumer::OffsetMembership => {
                    "offset receipt names a different original source"
                }
            },
            Self::RelationAdmission { .. } => {
                "geographic relation requires both complete source admissions"
            }
            Self::ChildBaseline { absorbing, .. } => {
                if *absorbing {
                    "absorbed child receipt exceeds its original baseline"
                } else {
                    "child source receipt exceeds its parent preparation"
                }
            }
            Self::OffsetBaseline { .. } => {
                "offset constructor is absent from the retained source baseline"
            }
            Self::BudgetPolicy { .. } => {
                "preparation receipt belongs to a different execution policy"
            }
            Self::OffsetAdmission { reason, .. } => match reason {
                PreparationBaselineReason::Work => {
                    "offset source preparation work receipt is missing"
                }
                PreparationBaselineReason::Workspace => {
                    "offset source preparation receipt is missing"
                }
            },
            Self::OffsetConstruction { .. } => "offset constructor source receipt is inconsistent",
        })
    }
}
impl std::error::Error for SourcePreparationError {}
