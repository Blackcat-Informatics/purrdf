// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_hash::error::{ErrorValue, StructuredError};

use super::{
    FramingDamage, ParityRecovery, ReadError, ReadFailure, RecoveryRefusal, Resynchronization,
};
use crate::v2::{
    DefinitionFindings, Generation, ItemKind, PayloadDamage, PayloadRefusal, RowFindings,
    UnknownFrameType,
};

impl StructuredError for ReadError {
    purrdf_hash::error_visit! {
        @record self { offset, failure } => "gts-v2-read-error" {
            "offset" => ByteCount(self.offset as u128),
            "failure" => Cause(&self.failure),
        }
    }
}

impl StructuredError for ReadFailure {
    purrdf_hash::error_visit! {
        Empty => "gts-v2-read-empty" {},
        Generation(generation) => "gts-v2-read-generation" { "generation" => Record(generation) },
        Framing(cause) => "gts-v2-read-framing" { "cause" => Cause(cause) },
        DetectionBytes { required, limit, cause } => "gts-read-detection-bytes" {
            "required" => ByteCount(*required as u128), "limit" => ByteCount(*limit as u128),
            "cause" => Value(cause.as_ref().map_or(ErrorValue::Absent, |cause| ErrorValue::Cause(cause))),
        },
        DetectionWork { required, limit, cause } => "gts-read-detection-work" {
            "required" => Unsigned(*required as u128), "limit" => Unsigned(*limit as u128),
            "cause" => Value(cause.as_ref().map_or(ErrorValue::Absent, |cause| ErrorValue::Cause(cause))),
        },
        DetectionDepth { required, limit, cause } => "gts-read-detection-depth" {
            "required" => Unsigned(*required as u128), "limit" => Unsigned(*limit as u128), "cause" => Cause(cause),
        },
        ItemDepth { required, limit, cause } => "gts-v2-read-item-depth" {
            "required" => Unsigned(*required as u128), "limit" => Unsigned(*limit as u128), "cause" => Cause(cause),
        },
        FirstGenerationCapacity(cause) => "gts-read-first-generation-capacity" { "cause" => Cause(cause) },
        ItemCapacity(limit) => "gts-v2-read-item-capacity" { "limit" => Unsigned(u128::from(*limit)) },
        Classification(reason) => "gts-v2-read-classification" { "reason" => Code(reason.code()) },
        Wire(cause) => "gts-v2-read-wire" { "cause" => Cause(cause) },
        Registry(version) => "gts-v2-read-registry" { "version" => Unsigned(u128::from(*version)) },
        UnavailableHash(algorithm) => "gts-v2-read-unavailable-hash" { "algorithm" => Unsigned(u128::from(algorithm.get())) },
        Lineage => "gts-v2-read-lineage" {},
        Item(kind) => "gts-v2-read-item" { "kind" => Record(kind) },
        Frame(kind) => "gts-v2-read-frame" { "type" => Unsigned(u128::from(*kind as u8)) },
        UnknownFrame(kind) => "gts-v2-read-unknown-frame" { "kind" => Record(kind) },
        BrokenLink => "gts-v2-read-broken-link" {},
        PayloadDamage(cause) => "gts-v2-read-payload-damage" { "cause" => Cause(cause) },
        PayloadRefusal(cause) => "gts-v2-read-payload-refusal" { "cause" => Cause(cause) },
        Segment(cause) => "gts-v2-read-segment" { "cause" => Cause(cause) },
        ParityRepair(cause) => "gts-v2-read-parity-repair" { "cause" => Cause(cause) },
        Commit => "gts-v2-read-invalid-commit" {},
        Content(cause) => "gts-v2-read-content" { "cause" => Cause(cause) },
        MissingContent => "gts-v2-read-missing-content" {},
        Definitions(findings) => "gts-v2-read-definitions" { "findings" => Record(findings) },
        Rows(findings) => "gts-v2-read-rows" { "findings" => Record(findings) },
        AnnotationWidth => "gts-v2-read-annotation-width" {},
        BlobDigest => "gts-v2-read-blob-digest" {},
        MissingUnit(name) => "gts-v2-read-missing-unit" { "unit" => Record(name) },
        UnitLength(name) => "gts-v2-read-unit-length" { "unit" => Record(name) },
        RebuildCapacity(name) => "gts-v2-read-rebuild-capacity" { "name" => Record(name) },
        Fold(cause) => "gts-v2-read-fold" { "cause" => Cause(cause) },
        Signature(cause) => "gts-v2-read-signature" { "cause" => Cause(cause) },
        RecoveredResync(recovery) => "gts-v2-read-recovered-resync" { "recovery" => Record(recovery) },
        RecoveredParity(recovery) => "gts-v2-read-recovered-parity" { "recovery" => Record(recovery) },
        RecoveryRefused(refusal) => "gts-v2-read-recovery-refused" { "refusal" => Record(refusal) },
        FramingTail { damage, cause } => "gts-v2-read-framing-tail" {
            "damage" => Record(damage),
            "cause" => Value(cause.as_ref().map_or(ErrorValue::Absent, |cause| ErrorValue::Cause(cause))),
        },
        OmittedContent => "gts-v2-read-omitted-content" {},
    }
}

impl StructuredError for Generation {
    purrdf_hash::error_visit! {
        First => "first_generation" {},
        Second => "second_generation" {},
        Unsupported(code) => "unsupported_generation" { "code" => Unsigned(u128::from(*code)) },
        Unrecognized => "unrecognized_generation" {},
    }
}

impl StructuredError for ItemKind {
    purrdf_hash::error_visit! {
        Header => "header" {},
        Frame => "frame" {},
        Offsets => "offsets" {},
        Locator => "locator" {},
        Resync => "resync" {},
        Unknown(code) => "unknown_item" { "code" => Unsigned(u128::from(*code)) },
    }
}

impl StructuredError for UnknownFrameType {
    purrdf_hash::error_visit! {
        Noncritical(code) => "unknown_frame_type" { "code" => Unsigned(u128::from(*code)) },
        Critical(code) => "unsupported_critical" { "code" => Unsigned(u128::from(*code)) },
    }
}

impl StructuredError for PayloadDamage {
    purrdf_hash::error_visit! {
        Malformed => "gts-v2-payload-malformed" {},
        DecodeFailed(cause) => "gts-v2-payload-decode" { "cause" => Cause(cause) },
        TrailingCompressedBytes => "gts-v2-payload-trailing-bytes" {},
        DecodedLength => "gts-v2-payload-decoded-length" {},
        PayloadDigest => "gts-v2-payload-digest" {},
    }
}

impl StructuredError for PayloadRefusal {
    purrdf_hash::error_visit! {
        StoredSize(length) => "gts-v2-payload-stored-size" { "length" => ByteCount(u128::from(*length)) },
        DecodedSize(length) => "gts-v2-payload-decoded-size" { "length" => ByteCount(u128::from(*length)) },
        TotalDecoded(length) => "gts-v2-payload-total-decoded" { "length" => ByteCount(u128::from(*length)) },
        UnknownCodec(code) => "gts-v2-payload-unknown-codec" { "code" => Unsigned(u128::from(*code)) },
        UnknownStorage(code) => "gts-v2-payload-unknown-storage" { "code" => Unsigned(u128::from(*code)) },
    }
}

impl StructuredError for DefinitionFindings {
    purrdf_hash::error_visit! {
        @record self { ill_formed_term, forward_reference, position_constraint, missing_values } =>
        "gts-v2-definition-findings" {
            "ill_formed_term" => Boolean(self.ill_formed_term),
            "forward_reference" => Boolean(self.forward_reference),
            "position_constraint" => Boolean(self.position_constraint),
            "missing_values" => List(self.missing_values.iter().map(|value| ErrorValue::Record(value))),
        }
    }
}

impl StructuredError for RowFindings {
    purrdf_hash::error_visit! {
        @record self { forward_reference, opaque_term, position_constraint } => "gts-v2-row-findings" {
            "forward_reference" => Boolean(self.forward_reference),
            "opaque_term" => Boolean(self.opaque_term),
            "position_constraint" => Boolean(self.position_constraint),
        }
    }
}

impl StructuredError for FramingDamage {
    purrdf_hash::error_visit! {
        TornTail { offset, length } => "gts-v2-torn-tail" {
            "offset" => ByteCount(*offset as u128), "length" => ByteCount(*length as u128),
        },
        Undelimitable { offset, length } => "gts-v2-undelimitable-input" {
            "offset" => ByteCount(*offset as u128), "length" => ByteCount(*length as u128),
        },
    }
}

impl StructuredError for Resynchronization {
    purrdf_hash::error_visit! {
        @record self { offset, damage } => "gts-v2-resynchronization" {
            "offset" => ByteCount(self.offset as u128),
            "damage" => Value(self.damage.as_ref().map_or(ErrorValue::Absent, |damage| ErrorValue::Record(damage))),
        }
    }
}

impl StructuredError for ParityRecovery {
    purrdf_hash::error_visit! {
        @record self { damage, offset, detached } => "gts-v2-parity-recovery" {
            "damage" => Record(&self.damage),
            "offset" => ByteCount(self.offset as u128),
            "detached" => Boolean(self.detached),
        }
    }
}

impl StructuredError for RecoveryRefusal {
    purrdf_hash::error_visit! {
        @record self { segment, position, offset, length, requested_total } => "gts-v2-recovery-refusal" {
            "segment" => Unsigned(u128::from(self.segment)),
            "position" => Unsigned(self.position as u128),
            "offset" => ByteCount(self.offset as u128),
            "length" => ByteCount(self.length as u128),
            "requested_total" => ByteCount(u128::from(self.requested_total)),
        }
    }
}
