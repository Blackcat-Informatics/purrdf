// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;

use super::fields::{Fields, OptionShape};
use super::payload::PayloadWire;
use super::{
    BLAKE3_256, CommitClaim, DIGEST_LENGTH, FRAME_FIELDS, LinkCheck, LinkState, MalformedReason,
    PayloadBudget, PayloadCheck, PayloadResourceError, REGISTRY_VERSION, WireError,
};

/// Frame types of GTS registry version 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum FrameType {
    /// Term dictionary records.
    Terms = 1,
    /// Dataset rows.
    Quads = 2,
    /// Reifier bindings.
    ReifierBindings = 3,
    /// Annotation rows.
    Annotations = 4,
    /// Declared empty graphs.
    EmptyGraphs = 5,
    /// Bindings.
    Bindings = 6,
    /// Suppressions.
    Suppress = 7,
    /// A blob chunk.
    Chunk = 8,
    /// A complete blob.
    Blob = 9,
    /// A blob reconstruction recipe.
    Recipe = 10,
    /// A value reconstruction recipe.
    Value = 11,
    /// An external object declaration.
    External = 12,
    /// A pack of members.
    Pack = 13,
    /// A commitment to the previous ordinary frames.
    Commit = 14,
    /// A directory of prior frames.
    Index = 15,
    /// Recovery parity.
    Parity = 16,
}

impl FrameType {
    /// Resolve a registry code this reader recognizes.
    #[must_use]
    pub const fn from_code(code: u64) -> Option<Self> {
        Some(match code {
            1 => Self::Terms,
            2 => Self::Quads,
            3 => Self::ReifierBindings,
            4 => Self::Annotations,
            5 => Self::EmptyGraphs,
            6 => Self::Bindings,
            7 => Self::Suppress,
            8 => Self::Chunk,
            9 => Self::Blob,
            10 => Self::Recipe,
            11 => Self::Value,
            12 => Self::External,
            13 => Self::Pack,
            14 => Self::Commit,
            15 => Self::Index,
            16 => Self::Parity,
            _ => return None,
        })
    }

    /// The registry's critical flag.
    #[must_use]
    pub const fn critical(self) -> bool {
        !matches!(self, Self::Commit | Self::Index | Self::Parity)
    }

    /// The registry's transparent flag.
    #[must_use]
    pub const fn transparent(self) -> bool {
        matches!(self, Self::Parity)
    }
}

/// An unimplemented frame type whose two flags satisfy the envelope grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownFrameType {
    /// The fold can continue past this frame.
    Noncritical(u64),
    /// The fold stops at this frame.
    Critical(u64),
}

/// Independent link and payload results and the first envelope diagnosis.
#[derive(Debug)]
#[non_exhaustive]
pub struct FrameCheck<'a> {
    /// Element 1 when it is a deterministic unsigned integer.
    pub frame_type: Option<u64>,
    /// The result for an unimplemented type in a newer registry.
    pub unknown_type: Option<UnknownFrameType>,
    /// The link result.
    pub link: LinkCheck,
    /// The stored and decoded payload results.
    pub payload: PayloadCheck<'a>,
    /// The first envelope diagnosis in wire order, then the codec constraint.
    pub malformed: Option<WireError>,
    /// The count and root of a commit whose content can be read.
    pub commit_claim: Option<CommitClaim<'a>>,
    /// A recognized parity frame violates its envelope rules.
    pub parity_envelope_damaged: bool,
}

/// Fields of a GTS v2 frame after the item kind, in wire order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameField {
    /// The frame type code.
    FrameType = 1,
    /// The critical bit.
    Critical = 2,
    /// The transparent bit.
    Transparent = 3,
    /// The content record.
    Content = 4,
    /// The decoded payload length.
    DecodedLength = 5,
    /// The decoded payload digest.
    PayloadDigest = 6,
    /// The storage form.
    Storage = 7,
    /// The stored payload byte string.
    Payload = 8,
    /// The stored frame identifier.
    Identifier = 9,
    /// The signature list.
    Signatures = 10,
}

/// The original encodings of fields in a complete P1 §3.1 frame envelope.
///
/// A framed value may contain malformed fields. The original encodings
/// let a reader check fields in wire order and retain the identity preimage.
#[derive(Clone, Copy, Debug)]
pub struct FrameEnvelope<'a> {
    bytes: &'a [u8],
    ends: [usize; FRAME_FIELDS],
}

pub(super) struct FrameWire<'a> {
    frame_type: Option<u64>,
    unknown_type: Option<UnknownFrameType>,
    link: LinkCheck,
    payload: PayloadWire<'a>,
    malformed: Option<WireError>,
    parity_envelope_damaged: bool,
}

impl FrameWire<'_> {
    pub(super) const fn passes_framing(&self) -> bool {
        self.malformed.is_none()
            && !self.parity_envelope_damaged
            && !matches!(self.link.state, LinkState::Broken)
    }
}

impl<'a> FrameEnvelope<'a> {
    pub(super) const fn new(bytes: &'a [u8], ends: [usize; FRAME_FIELDS]) -> Self {
        Self { bytes, ends }
    }

    /// The original CBOR bytes of one field, including its head.
    #[must_use]
    pub fn field(&self, field: FrameField) -> &'a [u8] {
        let index = field as usize;
        &self.bytes[self.start(field)..self.ends[index]]
    }

    pub(super) const fn start(&self, field: FrameField) -> usize {
        self.ends[field as usize - 1]
    }

    pub(super) fn read_field(&self, field: FrameField) -> Fields<'a> {
        Fields::at(self.field(field), self.start(field))
    }

    /// The exact contiguous encoding of elements 1 through 6.
    #[must_use]
    pub fn identity_fields(&self) -> &'a [u8] {
        &self.bytes[self.ends[0]..self.ends[FrameField::PayloadDigest as usize]]
    }

    /// Check the envelope, link and payload independently.
    ///
    /// Supply the segment's registry and the read's shared budget. A witness
    /// must be the immediately following frame with final framing.
    ///
    /// # Errors
    /// Returns a host resource refusal for the bounded output buffer.
    pub fn check(
        &self,
        predecessor: &[u8; DIGEST_LENGTH],
        witness: Option<&Self>,
        registry: NonZeroU64,
        budget: &mut PayloadBudget,
    ) -> Result<FrameCheck<'a>, PayloadResourceError> {
        self.check_prepared(self.check_wire(predecessor, witness, registry), budget)
    }

    pub(super) fn check_prepared(
        &self,
        wire: FrameWire<'a>,
        budget: &mut PayloadBudget,
    ) -> Result<FrameCheck<'a>, PayloadResourceError> {
        let payload = self.check_payload(wire.payload, budget)?;
        let known = wire.frame_type.and_then(FrameType::from_code);
        let mut malformed = wire
            .malformed
            .or_else(|| self.check_type_length(known))
            .or(payload.malformed);
        let mut commit_claim = None;
        if known == Some(FrameType::Commit)
            && malformed.is_none()
            && budget.admits_content(self.field(FrameField::Content).len() as u64)
        {
            match read_commit_claim(&mut self.read_field(FrameField::Content)) {
                Ok(claim) => commit_claim = Some(claim),
                Err(error) => malformed = Some(error),
            }
        }
        Ok(FrameCheck {
            frame_type: wire.frame_type,
            unknown_type: wire.unknown_type,
            link: wire.link,
            payload,
            malformed,
            commit_claim,
            parity_envelope_damaged: wire.parity_envelope_damaged,
        })
    }

    pub(super) fn passes_framing_test(
        &self,
        predecessor: &[u8; DIGEST_LENGTH],
        witness: Option<&Self>,
        registry: NonZeroU64,
    ) -> bool {
        self.check_wire(predecessor, witness, registry)
            .passes_framing()
    }

    pub(super) fn check_wire(
        &self,
        predecessor: &[u8; DIGEST_LENGTH],
        witness: Option<&Self>,
        registry: NonZeroU64,
    ) -> FrameWire<'a> {
        let frame_type = self.read_field(FrameField::FrameType).uint();
        let critical = self.read_field(FrameField::Critical).boolean();
        let transparent = self.read_field(FrameField::Transparent).boolean();
        let known = frame_type.ok().and_then(FrameType::from_code);
        let outer = self.check_outer_fields(frame_type, known, critical, transparent, registry);
        let unknown_type = match (frame_type, known, critical, transparent) {
            (Ok(code @ 1..), None, Ok(critical), Ok(_)) if registry.get() > REGISTRY_VERSION => {
                Some(if critical {
                    UnknownFrameType::Critical(code)
                } else {
                    UnknownFrameType::Noncritical(code)
                })
            }
            _ => None,
        };
        let signature_error = self.check_signatures(registry).err();
        let link = self.check_link(predecessor, witness);
        let payload = self.payload_wire(registry);
        let malformed = outer
            .err()
            .or(link.identity_error)
            .or_else(|| payload.grammar_error())
            .or(link.identifier_error)
            .or(signature_error);
        let parity_envelope_damaged = known == Some(FrameType::Parity)
            && (malformed.is_some()
                || self.field(FrameField::Storage) != [0x81, 1]
                || self.field(FrameField::Signatures) != [0x80]);
        FrameWire {
            frame_type: frame_type.ok(),
            unknown_type,
            link,
            payload,
            malformed,
            parity_envelope_damaged,
        }
    }

    fn check_outer_fields(
        &self,
        frame_type: Result<u64, WireError>,
        known: Option<FrameType>,
        critical: Result<bool, WireError>,
        transparent: Result<bool, WireError>,
        registry: NonZeroU64,
    ) -> Result<(), WireError> {
        let code = frame_type?;
        if code == 0 || (known.is_none() && registry.get() <= REGISTRY_VERSION) {
            return Err(WireError::new(
                MalformedReason::UnregisteredValue,
                self.start(FrameField::FrameType),
            ));
        }
        let critical = critical?;
        if known.is_some_and(|kind| kind.critical() != critical) {
            return Err(WireError::new(
                MalformedReason::ForbiddenCombination,
                self.start(FrameField::Critical),
            ));
        }
        let transparent = transparent?;
        if known.is_some_and(|kind| kind.transparent() != transparent) {
            return Err(WireError::new(
                MalformedReason::ForbiddenCombination,
                self.start(FrameField::Transparent),
            ));
        }
        Ok(())
    }

    fn check_type_length(&self, known: Option<FrameType>) -> Option<WireError> {
        let length = self.read_field(FrameField::DecodedLength).uint().ok()?;
        let invalid = match known? {
            FrameType::Chunk => length == 0,
            FrameType::External | FrameType::Commit => length != 0,
            _ => false,
        };
        invalid.then(|| {
            WireError::new(
                MalformedReason::OutOfRange,
                self.start(FrameField::DecodedLength),
            )
        })
    }

    fn check_signatures(&self, registry: NonZeroU64) -> Result<(), WireError> {
        let base = self.start(FrameField::Signatures);
        let bytes = self.field(FrameField::Signatures);
        let mut fields = Fields::at(bytes, base);
        let count = fields.array()?;
        let mut previous = None;
        for _ in 0..count {
            let start = fields.offset();
            fields.record(3)?;
            let algorithm = fields.offset();
            let code = fields.uint()?;
            if code == 0 || (code > 2 && registry.get() <= REGISTRY_VERSION) {
                return Err(WireError::new(
                    MalformedReason::UnregisteredValue,
                    algorithm,
                ));
            }
            fields.fixed_bytes::<DIGEST_LENGTH>()?;
            fields.bytes()?;
            let record = &bytes[start - base..fields.offset() - base];
            if previous.is_some_and(|previous| previous >= record) {
                return Err(WireError::new(MalformedReason::UnorderedOrRepeated, start));
            }
            previous = Some(record);
        }
        Ok(())
    }

    pub(super) fn check_identity_shape(&self) -> Result<(), WireError> {
        let frame_type = self.read_field(FrameField::FrameType).uint()?;
        self.read_field(FrameField::Critical).boolean()?;
        self.read_field(FrameField::Transparent).boolean()?;
        check_content_shape(frame_type, &mut self.read_field(FrameField::Content))?;
        self.read_field(FrameField::DecodedLength).uint()?;
        self.read_field(FrameField::PayloadDigest)
            .fixed_bytes::<DIGEST_LENGTH>()?;
        Ok(())
    }
}

/// The registry version 1 shapes, with value checks left to the content phase.
fn check_content_shape(frame_type: u64, fields: &mut Fields<'_>) -> Result<(), WireError> {
    let Some(frame_type) = FrameType::from_code(frame_type) else {
        return fields.closed_value();
    };
    match frame_type {
        FrameType::Terms => {
            fields.record(4)?;
            fields.uint()?;
            for _ in 0..fields.array()? {
                fields.record(2)?;
                fields.uint()?;
                fields.uint()?;
            }
            fields.uint()?;
            fields.uint()?;
        }
        FrameType::Quads
        | FrameType::ReifierBindings
        | FrameType::Annotations
        | FrameType::Pack => {
            fields.record(2)?;
            fields.uint()?;
            fields.uint()?;
        }
        FrameType::EmptyGraphs | FrameType::Bindings | FrameType::Suppress => {
            fields.record(1)?;
            fields.uint()?;
        }
        FrameType::Chunk | FrameType::Blob => {
            fields.record(if frame_type == FrameType::Chunk { 1 } else { 2 })?;
            fields.fixed_bytes::<DIGEST_LENGTH>()?;
            if frame_type == FrameType::Blob {
                media_type_shape(fields)?;
            }
        }
        FrameType::Recipe | FrameType::Value => {
            fields.record(if frame_type == FrameType::Recipe {
                4
            } else {
                3
            })?;
            fields.fixed_bytes::<DIGEST_LENGTH>()?;
            fields.uint()?;
            if frame_type == FrameType::Recipe {
                media_type_shape(fields)?;
            }
            fields.uint()?;
        }
        FrameType::External => {
            fields.record(3)?;
            fields.record(2)?;
            if fields.uint()? == BLAKE3_256 {
                fields.fixed_bytes::<DIGEST_LENGTH>()?;
            } else {
                fields.bytes()?;
            }
            fields.uint()?;
            media_type_shape(fields)?;
        }
        FrameType::Commit => {
            read_commit_claim(fields)?;
        }
        FrameType::Index => {
            fields.record(2)?;
            fields.uint()?;
            fields.fixed_bytes::<DIGEST_LENGTH>()?;
        }
        FrameType::Parity => {
            fields.record(5)?;
            fields.uint()?;
            fields.uint()?;
            fields.uint()?;
            fields.fixed_bytes::<DIGEST_LENGTH>()?;
            fields.fixed_bytes::<DIGEST_LENGTH>()?;
        }
    }
    Ok(())
}

fn read_commit_claim<'a>(fields: &mut Fields<'a>) -> Result<CommitClaim<'a>, WireError> {
    fields.record(2)?;
    Ok(CommitClaim {
        count: fields.uint()?,
        root: fields.fixed_bytes()?,
    })
}

fn media_type_shape(fields: &mut Fields<'_>) -> Result<(), WireError> {
    match fields.option_shape()? {
        OptionShape::Absent => {}
        OptionShape::Present => {
            fields.text()?;
        }
        OptionShape::Unknown { remaining, .. } => {
            // The unregistered tag is a value error. Its children must still
            // meet the closed grammar before these bytes can enter a link.
            for _ in 0..remaining {
                fields.closed_value()?;
            }
        }
    }
    Ok(())
}
