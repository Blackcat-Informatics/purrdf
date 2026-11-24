// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use core::num::NonZeroU64;
use std::collections::TryReserveError;

use purrdf_lex::cbor::StructuralError;

use super::fields::Fields;
use super::{
    FrameCheck, FrameEnvelope, FrameField, Item, LinkState, MalformedReason, PayloadState,
    REGISTRY_VERSION, WireError,
};

mod payload;
pub use payload::{
    DecodedTerms, FormedBudget, LiteralForm, TermDefinition, TermText, TypedValue, ValueReference,
};

const TERM_ID_END: u128 = u64::MAX as u128 + 1;
pub(super) const TRIPLE_KIND: u64 = 4;
pub(super) const LITERAL_VALUE_KIND: u64 = 6;
pub(super) const LITERAL_VALUE_REF_KIND: u64 = 7;
const LAST_TERM_KIND: u64 = LITERAL_VALUE_REF_KIND;

/// The declared order of the terms in a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TermOrder {
    /// The frame claims canonical term order.
    Canonical = 1,
    /// Term IDs follow arrival order.
    Arrival = 2,
}

/// A fault while interpreting a frame's content.
#[derive(Debug)]
#[non_exhaustive]
pub enum ContentError {
    /// The content record or decoded payload violates its grammar.
    Malformed(WireError),
    /// The segment uses a newer enumeration value.
    UnknownEnumeration {
        /// The code read from the item.
        code: u64,
        /// Position relative to the frame or decoded payload.
        offset: usize,
    },
    /// The read's formed-prefix capacity was exceeded.
    FormedCapacity(u64),
    /// The allocator refused storage for content.
    Resource(TryReserveError),
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(error) => error.fmt(f),
            Self::UnknownEnumeration { code, offset } => {
                write!(
                    f,
                    "GTS v2 content field at byte {offset} uses newer enumeration value {code}"
                )
            }
            Self::FormedCapacity(count) => {
                write!(f, "GTS v2 formed-byte capacity refused total {count}")
            }
            Self::Resource(error) => {
                write!(f, "could not reserve GTS v2 RDF content storage ({error})")
            }
        }
    }
}

impl std::error::Error for ContentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Malformed(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::UnknownEnumeration { .. } | Self::FormedCapacity(_) => None,
        }
    }
}

impl From<WireError> for ContentError {
    fn from(error: WireError) -> Self {
        Self::Malformed(error)
    }
}

purrdf_lex::variant_from!(ContentError { Resource(TryReserveError) });

impl purrdf_hash::error::StructuredError for ContentError {
    purrdf_hash::error_visit! {
        Malformed(cause) => "gts-v2-terms-malformed" { "cause" => Cause(cause) },
        UnknownEnumeration { code, offset } => "gts-v2-terms-unknown-enumeration" {
            "code" => Unsigned(u128::from(*code)),
            "offset" => ByteCount(*offset as u128),
        },
        FormedCapacity(count) => "gts-v2-terms-formed-capacity" { "count" => ByteCount(u128::from(*count)) },
        Resource(cause) => "gts-v2-terms-reservation" { "cause" => Cause(cause) },
    }
}

/// The admitted content record of a terms frame.
#[derive(Debug)]
pub struct TermsHeader<'a> {
    runs: Runs<'a>,
    registry: NonZeroU64,
    /// The declared ordering of the terms.
    pub order: TermOrder,
    /// The period of front-coding restarts inside each run.
    pub restart_group: NonZeroU64,
}

impl<'a> TermsHeader<'a> {
    /// Number of term runs in this content record.
    #[must_use]
    pub const fn run_count(&self) -> u64 {
        self.runs.count
    }

    /// Replay the validated run kind and positive count in dictionary order.
    pub fn runs(&self) -> impl Iterator<Item = (u64, NonZeroU64)> + use<'a> {
        self.runs.values()
    }
}

/// A terms frame's reservation in the segment dictionary.
#[derive(Debug)]
#[non_exhaustive]
pub struct TermReservation<'a> {
    /// First expected ID before observing this frame.
    pub start: u128,
    /// Exclusive end. It can equal 2^64 when the last ID is occupied.
    pub end: u128,
    /// The content check when the frame reaches interpretation.
    pub content: Option<Result<TermsHeader<'a>, ContentError>>,
    runs: Option<Runs<'a>>,
}

impl<'a> TermReservation<'a> {
    /// Replay the kinds and lengths of the independently admitted runs.
    pub fn runs(&self) -> impl Iterator<Item = (u64, NonZeroU64)> + use<'a> {
        self.runs.into_iter().flat_map(Runs::values)
    }

    pub(super) const fn run_count(&self) -> u64 {
        match self.runs {
            Some(runs) => runs.count,
            None => 0,
        }
    }

    pub(super) const fn reserved_end(&self) -> u128 {
        match self.runs {
            Some(runs) => runs.end,
            None => self.start,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Runs<'a> {
    count: u64,
    end: u128,
    fields: Fields<'a>,
}

impl<'a> Runs<'a> {
    fn values(self) -> impl Iterator<Item = (u64, NonZeroU64)> + use<'a> {
        let mut fields = self.fields;
        (0..self.count).map(move |_| {
            // The complete run list was admitted from these immutable bytes.
            fields.record(2).expect("admitted term run record");
            let kind = fields.uint().expect("admitted term run kind");
            let count = fields
                .positive(MalformedReason::OutOfRange)
                .expect("admitted term run count");
            (kind, count)
        })
    }
}

/// Reserve the IDs independently of link, payload and other field failures.
pub(super) fn reserve<'a>(
    frame: &FrameEnvelope<'a>,
    checked: &FrameCheck<'_>,
    start: u128,
    registry: NonZeroU64,
    content_readable: bool,
    stopped: bool,
) -> Result<TermReservation<'a>, StructuralError> {
    let mut observed = TermReservation {
        start,
        end: start,
        content: None,
        runs: None,
    };
    if !content_readable {
        return Ok(observed);
    }
    let mut fields = frame.read_field(FrameField::Content);
    let record_start = fields.offset();
    let mut base_offset = record_start;
    let claimed_base = fields.record(4).and_then(|()| {
        base_offset = fields.offset();
        fields.uint()
    });
    if claimed_base.is_err() {
        // A malformed field ahead of the runs can still leave a delimited
        // runs field. The framing walker locates it without admitting that field.
        if let Some(item) = Item::first(frame.field(FrameField::Content))?
            && let Some((offset, runs)) = item.array_element(1)?
            && let Ok(runs) = read_runs(
                &mut Fields::at(runs, record_start + offset),
                start,
                registry,
                &mut None,
            )
        {
            observed.end = runs.end;
            observed.runs = Some(runs);
        }
        return Ok(observed);
    }

    let mut unknown_run = None;
    let runs = read_runs(&mut fields, start, registry, &mut unknown_run);
    if let Ok(runs) = runs {
        observed.end = runs.end;
        observed.runs = Some(runs);
    }
    if !stopped
        && checked.malformed.is_none()
        && checked.link.state != LinkState::Broken
        && matches!(checked.payload.state, PayloadState::Verified(_))
    {
        observed.content = Some(
            if u128::from(claimed_base.expect("the first field was read")) != start {
                Err(WireError::new(MalformedReason::InconsistentCounts, base_offset).into())
            } else if let Some((code, offset)) = unknown_run {
                Err(ContentError::UnknownEnumeration { code, offset })
            } else {
                read_content(runs, &mut fields, registry)
            },
        );
    }
    Ok(observed)
}

fn read_runs<'a>(
    fields: &mut Fields<'a>,
    start: u128,
    registry: NonZeroU64,
    unknown: &mut Option<(u64, usize)>,
) -> Result<Runs<'a>, WireError> {
    let begin = fields.offset();
    let count = fields.array()?;
    if count == 0 {
        return Err(WireError::new(MalformedReason::OutOfRange, begin));
    }
    let first = *fields;
    let mut end = start;
    let mut previous = 0;
    for _ in 0..count {
        fields.record(2)?;
        let offset = fields.offset();
        let kind = fields.uint()?;
        if kind == 0 || (kind > LAST_TERM_KIND && registry.get() <= REGISTRY_VERSION) {
            return Err(WireError::new(MalformedReason::UnregisteredValue, offset));
        }
        if kind > LAST_TERM_KIND && unknown.is_none() {
            *unknown = Some((kind, offset));
        }
        if kind == previous {
            return Err(WireError::new(MalformedReason::NotDeterministic, offset));
        }
        previous = kind;
        end += u128::from(fields.positive(MalformedReason::OutOfRange)?.get());
    }
    if end > TERM_ID_END {
        return Err(WireError::new(MalformedReason::OutOfRange, begin));
    }
    Ok(Runs {
        count,
        end,
        fields: first,
    })
}

fn read_content<'a>(
    runs: Result<Runs<'a>, WireError>,
    fields: &mut Fields<'a>,
    registry: NonZeroU64,
) -> Result<TermsHeader<'a>, ContentError> {
    let runs = runs?;
    let order = read_enum(fields, 2, registry)?;
    let order = if order == 1 {
        TermOrder::Canonical
    } else {
        TermOrder::Arrival
    };
    let restart_group = fields.positive(MalformedReason::OutOfRange)?;
    Ok(TermsHeader {
        runs,
        registry,
        order,
        restart_group,
    })
}

pub(super) fn read_enum(
    fields: &mut Fields<'_>,
    last: u64,
    registry: NonZeroU64,
) -> Result<u64, ContentError> {
    let offset = fields.offset();
    let code = fields.positive(MalformedReason::UnregisteredValue)?.get();
    if code <= last {
        Ok(code)
    } else if registry.get() > REGISTRY_VERSION {
        Err(ContentError::UnknownEnumeration { code, offset })
    } else {
        Err(WireError::new(MalformedReason::UnregisteredValue, offset).into())
    }
}
