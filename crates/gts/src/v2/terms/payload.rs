// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::borrow::Cow;

use super::{ContentError, LAST_TERM_KIND, TermsHeader, read_enum};
use crate::v2::fields::{Fields, push};
use crate::v2::{DIGEST_LENGTH, MalformedReason, WireError};

/// The decoded text of a front-coded term or its original non-UTF-8 bytes.
#[derive(Debug, PartialEq, Eq)]
pub enum TermText<'a> {
    /// UTF-8 text.
    Text(Cow<'a, str>),
    /// Bytes that make the term unreadable.
    Invalid(Cow<'a, [u8]>),
}

impl TermText<'_> {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Text(value) => value.as_bytes(),
            Self::Invalid(bytes) => bytes,
        }
    }
}

/// The datatype reference or language and direction of an inline literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralForm {
    /// Dictionary ID of the datatype IRI.
    Typed(u64),
    /// Zero-based language-table index.
    Language(u64),
    /// Language-table index with left-to-right direction.
    LeftToRight(u64),
    /// Language-table index with right-to-left direction.
    RightToLeft(u64),
}

/// A digest and length naming a prior value declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueReference<'a> {
    /// Digest of the complete value.
    pub digest: &'a [u8; DIGEST_LENGTH],
    /// Length of the decoded value in bytes.
    pub length: u64,
}

/// An inline value in its typed storage form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedValue<'a> {
    /// Boolean.
    Boolean(bool),
    /// Unsigned integer.
    Unsigned(u64),
    /// Signed integer in the CBOR integer range.
    Integer(i128),
    /// UTC timestamp within years 0001 through 9999.
    Time {
        /// Seconds since the Unix epoch.
        seconds: i64,
        /// Fractional nanoseconds.
        nanoseconds: u32,
    },
    /// Bytes of a hex-binary literal.
    Octets(&'a [u8]),
}

/// One dictionary definition, before resolving references.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TermDefinition<'a> {
    /// IRI text.
    Iri(TermText<'a>),
    /// An inline lexical form and its datatype or language.
    Literal(TermText<'a>, LiteralForm),
    /// A segment-local blank label.
    Blank(&'a [u8]),
    /// Subject, predicate and object IDs in that order.
    Triple([u64; 3]),
    /// A value providing the lexical bytes and a literal form.
    LiteralRef(ValueReference<'a>, LiteralForm),
    /// A typed value.
    Value(TypedValue<'a>),
    /// A value providing the octets of a hex-binary literal.
    ValueRef(ValueReference<'a>),
}

impl TermDefinition<'_> {
    fn lexical(&self) -> Option<&TermText<'_>> {
        match self {
            Self::Iri(value) | Self::Literal(value, _) => Some(value),
            _ => None,
        }
    }
}

/// The definitions and language table of an admitted terms payload.
#[derive(Debug)]
#[non_exhaustive]
pub struct DecodedTerms<'a> {
    /// Definitions in ID order.
    pub terms: Vec<TermDefinition<'a>>,
    /// Language texts in their required order of first use.
    pub languages: Vec<&'a str>,
}

/// Read-wide capacity for copied front-coding prefixes.
#[derive(Clone, Debug)]
pub struct FormedBudget {
    limit: u64,
    charged: u64,
    refused: bool,
}

impl FormedBudget {
    /// Begin accounting with the caller's formed byte limit.
    #[must_use]
    pub const fn new(limit: u64) -> Self {
        Self {
            limit,
            charged: 0,
            refused: false,
        }
    }

    /// Prefix bytes charged to admitted terms frames.
    #[must_use]
    pub const fn charged(&self) -> u64 {
        self.charged
    }

    fn accept(&mut self, bytes: u128) -> Result<u64, ContentError> {
        let total = u128::from(self.charged) + bytes;
        if !self.refused && total <= u128::from(self.limit) {
            return Ok(u64::try_from(total).expect("a formed total below its u64 limit fits"));
        }
        self.refused = true;
        Err(ContentError::FormedCapacity(
            u64::try_from(total).unwrap_or(u64::MAX),
        ))
    }
}

impl TermsHeader<'_> {
    /// Decode the verified payload associated with this terms header.
    ///
    /// # Errors
    /// Returns the first fault in P3 content order or a formed capacity or
    /// host allocation refusal.
    pub fn decode<'a>(
        &self,
        payload: &'a [u8],
        budget: &mut FormedBudget,
    ) -> Result<DecodedTerms<'a>, ContentError> {
        let parsed = Parsed::read(payload, self.registry)?;
        parsed.check_counts(self)?;
        let charge = parsed.prefix_charge()?;
        let total = budget.accept(charge)?;
        let result = parsed.form(self.restart_group, self.registry);
        if !matches!(result, Err(ContentError::Resource(_))) {
            budget.charged = total;
        }
        result
    }
}

#[derive(Clone, Copy)]
struct List<'a> {
    count: u64,
    offset: usize,
    first: Fields<'a>,
}

impl<'a> List<'a> {
    fn read(
        fields: &mut Fields<'a>,
        mut read: impl FnMut(&mut Fields<'a>) -> Result<(), ContentError>,
    ) -> Result<Self, ContentError> {
        let offset = fields.offset();
        let count = fields.array()?;
        let first = *fields;
        for _ in 0..count {
            read(fields)?;
        }
        Ok(Self {
            count,
            offset,
            first,
        })
    }

    fn uints(fields: &mut Fields<'a>) -> Result<Self, ContentError> {
        Self::read(fields, |fields| {
            fields.uint().map(|_| ()).map_err(Into::into)
        })
    }

    fn blobs(fields: &mut Fields<'a>) -> Result<Self, ContentError> {
        Self::read(fields, |fields| {
            fields.bytes().map(|_| ()).map_err(Into::into)
        })
    }

    fn digests(fields: &mut Fields<'a>) -> Result<Self, ContentError> {
        Self::read(fields, |fields| {
            fields
                .fixed_bytes::<DIGEST_LENGTH>()
                .map(|_| ())
                .map_err(Into::into)
        })
    }

    fn check(self, count: u64) -> Result<(), ContentError> {
        if self.count != count {
            return Err(WireError::new(MalformedReason::InconsistentCounts, self.offset).into());
        }
        Ok(())
    }

    fn values<T, E>(
        self,
        mut read: impl FnMut(&mut Fields<'a>) -> Result<T, E>,
    ) -> impl Iterator<Item = Result<(usize, T), E>> {
        let mut fields = self.first;
        (0..self.count).map(move |_| {
            let offset = fields.offset();
            read(&mut fields).map(|value| (offset, value))
        })
    }
}

struct Forms<'a> {
    codes: List<'a>,
    datatypes: List<'a>,
    languages: List<'a>,
    typed: u64,
}

impl<'a> Forms<'a> {
    fn read(fields: &mut Fields<'a>, registry: NonZeroU64) -> Result<Self, ContentError> {
        let mut typed = 0;
        let codes = List::read(fields, |fields| {
            typed += u64::from(read_enum(fields, 4, registry)? == 1);
            Ok(())
        })?;
        Ok(Self {
            codes,
            datatypes: List::uints(fields)?,
            languages: List::uints(fields)?,
            typed,
        })
    }

    fn check(&self, count: u64) -> Result<(), ContentError> {
        self.codes.check(count)?;
        self.datatypes.check(self.typed)?;
        self.languages.check(count - self.typed)
    }

    fn values(&self) -> impl Iterator<Item = Result<LiteralForm, WireError>> + use<'a> {
        let mut datatypes = self.datatypes.values(Fields::uint);
        let mut languages = self.languages.values(Fields::uint);
        self.codes.values(Fields::uint).map(move |code| {
            // Counts match the form list. Each code consumes one entry of
            // precisely the companion column declared by that code.
            let (_, code) = code?;
            if code == 1 {
                return Ok(LiteralForm::Typed(
                    datatypes.next().expect("admitted datatype column")?.1,
                ));
            }
            let (_, index) = languages.next().expect("admitted language column")?;
            Ok(match code {
                2 => LiteralForm::Language(index),
                3 => LiteralForm::LeftToRight(index),
                4 => LiteralForm::RightToLeft(index),
                _ => unreachable!("literal form codes were admitted"),
            })
        })
    }
}

struct Front<'a> {
    prefixes: List<'a>,
    lengths: List<'a>,
    suffixes: &'a [u8],
}

impl<'a> Front<'a> {
    fn read(fields: &mut Fields<'a>) -> Result<Self, ContentError> {
        Ok(Self {
            prefixes: List::uints(fields)?,
            lengths: List::uints(fields)?,
            suffixes: fields.bytes()?,
        })
    }

    fn form(
        &self,
        restart: NonZeroU64,
        mut forms: Option<impl Iterator<Item = Result<LiteralForm, WireError>>>,
        output: &mut Vec<TermDefinition<'a>>,
    ) -> Result<(), ContentError> {
        let suffix_total = sum(self.lengths)?;
        if suffix_total != self.suffixes.len() as u128 {
            return Err(
                WireError::new(MalformedReason::InconsistentCounts, self.lengths.offset).into(),
            );
        }
        let mut suffixes = self.suffixes;
        for (index, (prefix, length)) in self
            .prefixes
            .values(Fields::uint)
            .zip(self.lengths.values(Fields::uint))
            .enumerate()
        {
            let (offset, prefix) = prefix?;
            let (_, length) = length?;
            let previous = if index == 0 {
                &[][..]
            } else {
                output
                    .last()
                    .and_then(TermDefinition::lexical)
                    .expect("preceding text of this run")
                    .bytes()
            };
            let is_restart = (index as u64).is_multiple_of(restart.get());
            if index != 0 && prefix > previous.len() as u64 {
                return Err(WireError::new(MalformedReason::OutOfRange, offset).into());
            }
            if is_restart && prefix != 0 {
                return Err(WireError::new(MalformedReason::ForbiddenCombination, offset).into());
            }
            // The prefix fits the preceding text. Suffix lengths sum to
            // the length of the original addressable slice.
            let prefix = usize::try_from(prefix).expect("prefix fits the preceding bytes");
            let length = usize::try_from(length).expect("suffix fits the admitted bytes");
            let (suffix, rest) = suffixes.split_at(length);
            suffixes = rest;
            if !is_restart
                && suffix
                    .first()
                    .is_some_and(|byte| previous.get(prefix) == Some(byte))
            {
                return Err(WireError::new(MalformedReason::NotDeterministic, offset).into());
            }
            let text = form_text(&previous[..prefix], suffix)?;
            let term = if let Some(forms) = forms.as_mut() {
                TermDefinition::Literal(text, forms.next().expect("admitted literal forms")?)
            } else {
                TermDefinition::Iri(text)
            };
            push(output, term)?;
        }
        Ok(())
    }
}

fn form_text<'a>(prefix: &[u8], suffix: &'a [u8]) -> Result<TermText<'a>, ContentError> {
    if prefix.is_empty() {
        return Ok(match core::str::from_utf8(suffix) {
            Ok(text) => TermText::Text(Cow::Borrowed(text)),
            Err(_) => TermText::Invalid(Cow::Borrowed(suffix)),
        });
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(prefix.len().saturating_add(suffix.len()))
        .map_err(ContentError::Resource)?;
    bytes.extend_from_slice(prefix);
    bytes.extend_from_slice(suffix);
    Ok(match String::from_utf8(bytes) {
        Ok(text) => TermText::Text(Cow::Owned(text)),
        Err(error) => TermText::Invalid(Cow::Owned(error.into_bytes())),
    })
}

fn sum(list: List<'_>) -> Result<u128, ContentError> {
    list.values(Fields::uint)
        .try_fold(0, |sum, value| Ok(sum + u128::from(value?.1)))
}

struct Block<'a> {
    offset: usize,
    kind: u64,
    body: BlockBody<'a>,
}

enum BlockBody<'a> {
    Iri(Front<'a>),
    Literal(Front<'a>, Forms<'a>),
    Blank(List<'a>),
    Triple([List<'a>; 3]),
    LiteralRef(References<'a>, Forms<'a>),
    Value(List<'a>),
    ValueRef(References<'a>),
}

struct References<'a> {
    digests: List<'a>,
    lengths: List<'a>,
}

impl<'a> References<'a> {
    fn read(fields: &mut Fields<'a>) -> Result<Self, ContentError> {
        Ok(Self {
            digests: List::digests(fields)?,
            lengths: List::uints(fields)?,
        })
    }

    fn check(&self, count: u64) -> Result<(), ContentError> {
        self.digests.check(count)?;
        self.lengths.check(count)
    }

    fn values(&self) -> impl Iterator<Item = Result<ValueReference<'a>, WireError>> + use<'a> {
        self.digests
            .values(Fields::fixed_bytes::<DIGEST_LENGTH>)
            .zip(self.lengths.values(Fields::uint))
            .map(|(digest, length)| {
                Ok(ValueReference {
                    digest: digest?.1,
                    length: length?.1,
                })
            })
    }
}

impl<'a> Block<'a> {
    fn read(fields: &mut Fields<'a>, registry: NonZeroU64) -> Result<Self, ContentError> {
        let (offset, count, kind) = variant(fields, LAST_TERM_KIND, registry)?;
        let expected = match kind {
            1 | 4 => 4,
            2 => 7,
            3 | 6 => 2,
            5 => 6,
            7 => 3,
            _ => unreachable!("term block kind was admitted"),
        };
        if count != expected {
            return Err(WireError::new(MalformedReason::WrongLength, offset).into());
        }
        let body = match kind {
            1 => BlockBody::Iri(Front::read(fields)?),
            2 => BlockBody::Literal(Front::read(fields)?, Forms::read(fields, registry)?),
            3 => BlockBody::Blank(List::blobs(fields)?),
            4 => BlockBody::Triple([
                List::uints(fields)?,
                List::uints(fields)?,
                List::uints(fields)?,
            ]),
            5 => BlockBody::LiteralRef(References::read(fields)?, Forms::read(fields, registry)?),
            6 => BlockBody::Value(List::read(fields, |reader| {
                read_value(reader, registry).map(|_| ())
            })?),
            7 => BlockBody::ValueRef(References::read(fields)?),
            _ => unreachable!("term block kind was admitted"),
        };
        Ok(Self { offset, kind, body })
    }

    fn check_counts(&self, count: u64) -> Result<(), ContentError> {
        match &self.body {
            BlockBody::Iri(front) | BlockBody::Literal(front, _) => {
                front.prefixes.check(count)?;
                front.lengths.check(count)?;
                if let BlockBody::Literal(_, forms) = &self.body {
                    forms.check(count)?;
                }
            }
            BlockBody::Blank(labels) | BlockBody::Value(labels) => labels.check(count)?,
            BlockBody::Triple(columns) => {
                for column in columns {
                    column.check(count)?;
                }
            }
            BlockBody::LiteralRef(references, forms) => {
                references.check(count)?;
                forms.check(count)?;
            }
            BlockBody::ValueRef(references) => references.check(count)?,
        }
        Ok(())
    }

    fn form(
        &self,
        restart: NonZeroU64,
        registry: NonZeroU64,
        terms: &mut Vec<TermDefinition<'a>>,
    ) -> Result<(), ContentError> {
        match &self.body {
            BlockBody::Iri(front) => front.form(restart, None::<core::iter::Empty<_>>, terms)?,
            BlockBody::Literal(front, forms) => front.form(restart, Some(forms.values()), terms)?,
            BlockBody::Blank(labels) => {
                for label in labels.values(Fields::bytes) {
                    push(terms, TermDefinition::Blank(label?.1))?;
                }
            }
            BlockBody::Triple([subjects, predicates, objects]) => {
                for ((subject, predicate), object) in subjects
                    .values(Fields::uint)
                    .zip(predicates.values(Fields::uint))
                    .zip(objects.values(Fields::uint))
                {
                    push(
                        terms,
                        TermDefinition::Triple([subject?.1, predicate?.1, object?.1]),
                    )?;
                }
            }
            BlockBody::LiteralRef(references, forms) => {
                for (reference, form) in references.values().zip(forms.values()) {
                    push(terms, TermDefinition::LiteralRef(reference?, form?))?;
                }
            }
            BlockBody::Value(values) => {
                for value in values.values(|reader| read_value(reader, registry)) {
                    push(terms, TermDefinition::Value(value?.1))?;
                }
            }
            BlockBody::ValueRef(references) => {
                for reference in references.values() {
                    push(terms, TermDefinition::ValueRef(reference?))?;
                }
            }
        }
        Ok(())
    }

    fn language_indexes(&self) -> Option<List<'a>> {
        match &self.body {
            BlockBody::Literal(_, forms) | BlockBody::LiteralRef(_, forms) => Some(forms.languages),
            _ => None,
        }
    }

    fn prefixes(&self) -> Option<List<'a>> {
        match &self.body {
            BlockBody::Iri(front) | BlockBody::Literal(front, _) => Some(front.prefixes),
            _ => None,
        }
    }
}

fn variant(
    fields: &mut Fields<'_>,
    last: u64,
    registry: NonZeroU64,
) -> Result<(usize, u64, u64), ContentError> {
    let offset = fields.offset();
    let count = fields.array()?;
    if count == 0 {
        return Err(WireError::new(MalformedReason::WrongLength, offset).into());
    }
    Ok((offset, count, read_enum(fields, last, registry)?))
}

fn read_value<'a>(
    fields: &mut Fields<'a>,
    registry: NonZeroU64,
) -> Result<TypedValue<'a>, ContentError> {
    let (offset, count, tag) = variant(fields, 5, registry)?;
    if count != if tag == 4 { 3 } else { 2 } {
        return Err(WireError::new(MalformedReason::WrongLength, offset).into());
    }
    Ok(match tag {
        1 => TypedValue::Boolean(fields.boolean()?),
        2 => TypedValue::Unsigned(fields.uint()?),
        3 => TypedValue::Integer(fields.int()?),
        4 => {
            let offset = fields.offset();
            let seconds = fields.int()?;
            if !(-62_135_596_800..=253_402_300_799).contains(&seconds) {
                return Err(WireError::new(MalformedReason::OutOfRange, offset).into());
            }
            let offset = fields.offset();
            let nanoseconds = fields.uint()?;
            if nanoseconds >= 1_000_000_000 {
                return Err(WireError::new(MalformedReason::OutOfRange, offset).into());
            }
            TypedValue::Time {
                seconds: i64::try_from(seconds).expect("admitted years fit i64 seconds"),
                nanoseconds: u32::try_from(nanoseconds).expect("admitted nanoseconds fit u32"),
            }
        }
        5 => TypedValue::Octets(fields.bytes()?),
        _ => unreachable!("typed value kind was admitted"),
    })
}

struct Parsed<'a> {
    blocks_offset: usize,
    blocks: Vec<Block<'a>>,
    languages_offset: usize,
    languages: Vec<&'a str>,
}

impl<'a> Parsed<'a> {
    fn read(bytes: &'a [u8], registry: NonZeroU64) -> Result<Self, ContentError> {
        let mut fields = Fields::new(bytes);
        fields.record(2)?;
        let blocks_offset = fields.offset();
        let count = fields.array()?;
        let mut blocks = Vec::new();
        for _ in 0..count {
            push(&mut blocks, Block::read(&mut fields, registry)?)?;
        }
        let languages_offset = fields.offset();
        let count = fields.array()?;
        let mut languages = Vec::new();
        for _ in 0..count {
            push(&mut languages, fields.text()?)?;
        }
        fields.finish()?;
        Ok(Self {
            blocks_offset,
            blocks,
            languages_offset,
            languages,
        })
    }

    fn check_counts(&self, header: &TermsHeader<'_>) -> Result<(), ContentError> {
        if self.blocks.len() as u64 != header.run_count() {
            return Err(
                WireError::new(MalformedReason::InconsistentCounts, self.blocks_offset).into(),
            );
        }
        for ((_, count), block) in header.runs().zip(&self.blocks) {
            block.check_counts(count.get())?;
        }
        for ((kind, _), block) in header.runs().zip(&self.blocks) {
            if block.kind != kind {
                return Err(
                    WireError::new(MalformedReason::ForbiddenCombination, block.offset).into(),
                );
            }
        }
        Ok(())
    }

    fn prefix_charge(&self) -> Result<u128, ContentError> {
        self.blocks
            .iter()
            .filter_map(Block::prefixes)
            .try_fold(0, |total, prefixes| Ok(total + sum(prefixes)?))
    }

    fn form(
        self,
        restart: NonZeroU64,
        registry: NonZeroU64,
    ) -> Result<DecodedTerms<'a>, ContentError> {
        let mut terms = Vec::new();
        for block in &self.blocks {
            block.form(restart, registry, &mut terms)?;
        }
        self.check_languages()?;
        Ok(DecodedTerms {
            terms,
            languages: self.languages,
        })
    }

    fn check_languages(&self) -> Result<(), ContentError> {
        let indices = || {
            self.blocks
                .iter()
                .filter_map(Block::language_indexes)
                .flat_map(|list| list.values(Fields::uint))
        };
        let count = self.languages.len() as u64;
        for index in indices() {
            let (offset, index) = index?;
            if index >= count {
                return Err(WireError::new(MalformedReason::OutOfRange, offset).into());
            }
        }
        let mut seen = 0;
        for index in indices() {
            let (_, index) = index?;
            if index == seen {
                seen += 1;
            } else if index > seen {
                return Err(self.language_order_error());
            }
        }
        if seen != count {
            return Err(self.language_order_error());
        }
        let mut unique = crate::FastSet::default();
        unique
            .try_reserve(self.languages.len())
            .map_err(ContentError::Resource)?;
        for language in &self.languages {
            if !unique.insert(language) {
                return Err(self.language_order_error());
            }
        }
        Ok(())
    }

    fn language_order_error(&self) -> ContentError {
        WireError::new(MalformedReason::NotDeterministic, self.languages_offset).into()
    }
}
