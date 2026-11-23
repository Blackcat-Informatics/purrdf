// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::io::Cursor;

use purrdf_lex::cbor::{Limits, StructuralError, StructuralRange, head};

use super::fields::Fields;
use super::frame::FrameEnvelope;
use super::{
    DIGEST_LENGTH, FRAME_FIELDS, FRAME_KIND, GENERATION, HEADER_KIND, Header, MalformedReason,
    REGISTRY_VERSION, RESYNC_KIND, WireError,
};

const LOCATOR_KIND: u64 = 3;
const OFFSETS_KIND: u64 = 4;

/// The generation identified by the first complete item under P1 §2.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Generation {
    /// A version 1 map header, bare or under the self-describe tag.
    First,
    /// A version 2 header or resync opener. Its grammar is checked separately.
    Second,
    /// A generation identified by the opener grammar but unavailable here.
    Unsupported(u64),
    /// An item that does not identify a supported or future GTS generation.
    Unrecognized,
}

/// Structural class of a complete item in a v2 segment under P1 §2.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ItemKind {
    /// A segment header, possibly with invalid fields.
    Header,
    /// A complete 11-field frame envelope. Its fields have yet to be checked.
    Frame,
    /// An offsets item.
    Offsets,
    /// A locator item.
    Locator,
    /// A resync opener, possibly with invalid fields.
    Resync,
    /// An item kind this reader does not hold, in a newer registry.
    Unknown(u64),
}

/// One structurally delimited CBOR item, borrowing its exact original bytes.
///
/// Framing walks heads and container extents. Decoders judge the individual
/// fields, including UTF-8 and deterministic encoding, after framing.
#[derive(Clone, Copy, Debug)]
pub struct Item<'a> {
    bytes: &'a [u8],
    range: StructuralRange,
}

impl<'a> Item<'a> {
    /// Delimit the first item, leaving subsequent bytes untouched.
    ///
    /// # Errors
    /// Returns the native CBOR framing refusal. Empty input returns
    /// `Ok(None)`. Each opening head occupies at least one input byte, so
    /// the framing walk can admit every representable nesting depth.
    pub fn first(bytes: &'a [u8]) -> Result<Option<Self>, StructuralError> {
        let mut source = Cursor::new(bytes);
        let limits = Limits {
            max_depth: bytes.len(),
        };
        let Some(range) = StructuralRange::scan(&mut source, 0, bytes.len() as u64, limits)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            bytes: item_slice(bytes, range),
            range,
        }))
    }

    /// The extent and encoding of this item.
    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    pub(super) fn array_element(
        self,
        index: u64,
    ) -> Result<Option<(usize, &'a [u8])>, StructuralError> {
        let mut source = Cursor::new(self.bytes);
        let Some(mut children) = self.range.array(&mut source)? else {
            return Ok(None);
        };
        for _ in 0..index {
            if children.next(&mut source)?.is_none() {
                return Ok(None);
            }
        }
        let Some(child) = children.next(&mut source)? else {
            return Ok(None);
        };
        let start = usize::try_from(child.start()).expect("a child begins within the source slice");
        Ok(Some((start, item_slice(self.bytes, child))))
    }

    /// Check the closed grammar of an otherwise untyped item under P1-10.
    ///
    /// # Errors
    /// Returns the first closed-grammar violation and its item-relative offset.
    pub fn validate_grammar(self) -> Result<(), WireError> {
        Fields::new(self.bytes).closed_value()
    }

    /// Classify an item after the file's initial generation check.
    ///
    /// Header and resync openers are recognized before checking the form
    /// of their heads. Their field grammar is checked when they are decoded.
    ///
    /// # Errors
    /// Returns the reason an item fails classification and is opaque.
    pub fn classify(self, registry_version: NonZeroU64) -> Result<ItemKind, MalformedReason> {
        let prefix = self.record_prefix().ok_or(MalformedReason::WrongType)?;
        match prefix.kind {
            HEADER_KIND => Ok(ItemKind::Header),
            RESYNC_KIND => Ok(ItemKind::Resync),
            _ if !prefix.shortest => Err(MalformedReason::NotDeterministic),
            0 => Err(MalformedReason::UnregisteredValue),
            FRAME_KIND if prefix.count == Some(FRAME_FIELDS as u64) => Ok(ItemKind::Frame),
            FRAME_KIND => Err(MalformedReason::WrongLength),
            OFFSETS_KIND => Ok(ItemKind::Offsets),
            LOCATOR_KIND => Ok(ItemKind::Locator),
            kind if registry_version.get() > REGISTRY_VERSION => Ok(ItemKind::Unknown(kind)),
            _ => Err(MalformedReason::UnregisteredValue),
        }
    }

    /// Decide whether this complete item passes the P1 §6.7 framing test.
    ///
    /// A readable segment supplies its declared registry and effective
    /// predecessor. Pass `None` for an unreadable opener or unavailable
    /// links. A witness stands directly after this item. If recovery discards
    /// the witness, check the frame again with no witness. This test reads
    /// envelope fields and links without decoding payloads or checking claims.
    ///
    /// # Errors
    /// Returns a native structural failure while locating frame fields.
    pub fn passes_framing_test(
        self,
        segment: Option<(NonZeroU64, &[u8; DIGEST_LENGTH])>,
        witness: Option<&FrameEnvelope<'a>>,
    ) -> Result<bool, StructuralError> {
        let registry = segment.map_or(NonZeroU64::MAX, |(registry, _)| registry);
        let Ok(kind) = self.classify(registry) else {
            return Ok(false);
        };
        match (kind, segment) {
            (ItemKind::Header, _) => Ok(Header::decode(self).is_ok()),
            (ItemKind::Resync, _) => Ok(Header::decode_resync(self).is_ok()),
            (ItemKind::Frame, Some((registry, predecessor))) => Ok(self
                .frame()?
                .is_some_and(|frame| frame.passes_framing_test(predecessor, witness, registry))),
            (ItemKind::Offsets, Some(_)) => Ok(self.offsets_match(&[]).is_ok()),
            (ItemKind::Locator, Some(_)) => Ok(self.locator_distance().is_ok()),
            _ => Ok(true),
        }
    }

    pub(super) fn offsets_match(self, observed: &[usize]) -> Result<bool, WireError> {
        let mut fields = Fields::new(self.bytes);
        fields.record(2)?;
        fields.fixed_uint(OFFSETS_KIND, MalformedReason::WrongType)?;
        let count = fields.array()?;
        let mut matches = count == observed.len() as u64;
        let mut lengths = observed.iter().copied();
        for _ in 0..count {
            let length = fields.uint()?;
            matches &= lengths
                .next()
                .is_some_and(|observed| observed as u64 == length);
        }
        Ok(matches)
    }

    pub(super) fn locator_distance(self) -> Result<u64, WireError> {
        let mut fields = Fields::new(self.bytes);
        fields.record(2)?;
        fields.fixed_uint(LOCATOR_KIND, MalformedReason::WrongType)?;
        let offset = fields.offset();
        let mut distance = fields.remaining();
        let value = head::read_argument(&mut distance, head::UNSIGNED)
            .ok_or_else(|| WireError::new(MalformedReason::WrongType, offset))?;
        if fields.remaining().first() != Some(&0x1b) {
            return Err(WireError::new(MalformedReason::NotDeterministic, offset));
        }
        Ok(value)
    }

    /// Locate the fields of a complete P1 §3.1 frame envelope.
    ///
    /// The original field bytes borrow from the framed item. A frame with
    /// malformed fields is still an envelope under the classification rules.
    ///
    /// # Errors
    /// Propagates native structural errors while visiting the fields.
    pub fn frame(self) -> Result<Option<FrameEnvelope<'a>>, StructuralError> {
        if self.classify(NonZeroU64::MIN) != Ok(ItemKind::Frame) {
            return Ok(None);
        }

        let mut source = Cursor::new(self.bytes);
        let mut children = self
            .range
            .array(&mut source)?
            .expect("a classified frame is an array");
        let mut ends = [0; FRAME_FIELDS];
        for end in &mut ends {
            let range = children
                .next(&mut source)?
                .expect("a classified frame has eleven fields");
            *end = usize::try_from(range.end()).expect("a field ends within the source slice");
        }
        Ok(Some(FrameEnvelope::new(self.bytes, ends)))
    }

    /// Identify a first item's format by its declared generation.
    ///
    /// This examines the opener only. File recovery and segment grammar
    /// checks operate on the original bytes separately.
    ///
    /// # Errors
    /// Propagates native structural errors while visiting the opener.
    pub fn generation(self) -> Result<Generation, StructuralError> {
        if let Some(prefix) = self.record_prefix() {
            if !matches!(prefix.kind, HEADER_KIND | RESYNC_KIND) {
                return Ok(Generation::Unrecognized);
            }
            let mut tail = prefix.tail;
            return Ok(match head::read_argument(&mut tail, head::UNSIGNED) {
                Some(GENERATION) => Generation::Second,
                Some(generation @ 3..) => Generation::Unsupported(generation),
                _ => Generation::Unrecognized,
            });
        }

        let mut source = Cursor::new(self.bytes);
        let map = match self.range.tag(&mut source)? {
            None => self.range,
            Some((crate::wire::SELF_DESCRIBE_TAG, child)) => child,
            Some(_) => return Ok(Generation::Unrecognized),
        };
        let Some(mut fields) = map.map(&mut source)? else {
            return Ok(Generation::Unrecognized);
        };
        while let Some(key) = fields.next(&mut source)? {
            let value = fields.next(&mut source)?;
            if is_version_key(item_slice(self.bytes, key)) {
                return Ok(match value.and_then(|item| unsigned(self.bytes, item)) {
                    Some(1) => Generation::First,
                    Some(generation) => Generation::Unsupported(generation),
                    None => Generation::Unrecognized,
                });
            }
        }
        Ok(Generation::Unrecognized)
    }

    fn record_prefix(self) -> Option<RecordPrefix<'a>> {
        let mut fields = self.bytes;
        let count = if let Some(tail) = fields.strip_prefix(&[0x9f]) {
            fields = tail;
            None
        } else {
            Some(head::read_argument(&mut fields, head::ARRAY)?)
        };
        let mut tail = fields;
        let kind = head::read_argument(&mut tail, head::UNSIGNED)?;
        let shortest = count
            .is_some_and(|count| self.bytes.len() - fields.len() == head::head_len(count))
            && fields.len() - tail.len() == head::head_len(kind);
        Some(RecordPrefix {
            count,
            kind,
            shortest,
            tail,
        })
    }
}

struct RecordPrefix<'a> {
    count: Option<u64>,
    kind: u64,
    shortest: bool,
    tail: &'a [u8],
}

fn item_slice(bytes: &[u8], range: StructuralRange) -> &[u8] {
    // These extents were obtained from a Cursor over this slice and validated
    // against its addressable length, so both bounds fit usize.
    let start = usize::try_from(range.start()).expect("INVARIANT: item begins within the slice");
    let end = usize::try_from(range.end()).expect("INVARIANT: item ends within the slice");
    &bytes[start..end]
}

fn unsigned(bytes: &[u8], range: StructuralRange) -> Option<u64> {
    head::read_argument(&mut item_slice(bytes, range), head::UNSIGNED)
}

fn is_version_key(mut bytes: &[u8]) -> bool {
    if let Some(text) = head::read_text(&mut bytes) {
        return text == "v";
    }
    // Framing already established the string's extent and the final break.
    // Compare the chunks against the one-byte key without flattening them.
    let Some(mut chunks) = bytes
        .strip_prefix(&[0x7f])
        .and_then(|body| body.strip_suffix(&[0xff]))
    else {
        return false;
    };
    let mut key = "v";
    while !chunks.is_empty() {
        let Some(chunk) = head::read_text(&mut chunks) else {
            return false;
        };
        let Some(remaining) = key.strip_prefix(chunk) else {
            return false;
        };
        key = remaining;
    }
    key.is_empty()
}
