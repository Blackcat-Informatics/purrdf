// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original WKT grammar decisions, tokens and coordinate context.

use crate::{CoordDim, GeometryKind};
use purrdf_lex::allocation::{Admission, Memory, StorageError};

/// Native reason a WKT grammar production was refused.
/// The default owns its tokens; borrowed text is used during admitted construction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WktReason<T = String> {
    /// Text follows the one complete geometry.
    TrailingText {
        /// Original bounded source preview.
        found: T,
    },
    /// A production requires a particular byte.
    ExpectedByte {
        /// Original required byte.
        expected: u8,
        /// Grammar production being read.
        production: &'static str,
        /// Original bounded source preview.
        found: T,
    },
    /// No number begins at the ordinate position.
    ExpectedNumber {
        /// Original bounded source preview.
        found: T,
    },
    /// The original maximal numeric token has no exact supported value.
    InvalidNumber {
        /// Original numeric token.
        lexical: T,
    },
    /// An opening CRS bracket has no closing bracket.
    UnclosedCrs,
    /// An explicit CRS prefix contains no identifier.
    EmptyCrs,
    /// The original word is not a geometry keyword.
    UnknownKeyword {
        /// Original maximal word, possibly empty.
        word: T,
        /// Following source when no word was present.
        preview: Option<T>,
    },
    /// An unknown word follows a geometry keyword.
    BadTag {
        /// Original geometry kind.
        kind: GeometryKind,
        /// Original refused word.
        word: T,
    },
    /// An unknown word follows a dimension tag.
    BadTagFollower {
        /// Original dimension-tag spelling.
        tag: T,
        /// Original refused word.
        word: T,
    },
    /// Empty parentheses are neither EMPTY nor a member list.
    EmptyList {
        /// Original grammar production.
        production: &'static str,
    },
    /// Another number follows the dimension's final ordinate.
    ExtraOrdinate {
        /// Original declared coordinate dimension.
        dimension: CoordDim,
    },
}

impl purrdf_hash::error::StructuredError for WktReason {
    purrdf_hash::error_visit! {
        TrailingText { found } => "wkt-trailing-text" { "found" => Text(found.as_ref()) },
        ExpectedByte { expected, production, found } => "wkt-expected-byte" {
            "expected_byte" => Unsigned(u128::from(*expected)), "production" => Code(production), "found" => Text(found.as_ref())
        },
        ExpectedNumber { found } => "wkt-expected-number" { "found" => Text(found.as_ref()) },
        InvalidNumber { lexical } => "wkt-invalid-number" { "lexical" => Text(lexical.as_ref()) },
        UnclosedCrs => "wkt-unclosed-crs" {},
        EmptyCrs => "wkt-empty-crs" {},
        UnknownKeyword { word, preview } => "wkt-unknown-keyword" {
            "word" => Text(word.as_ref()), "preview" => Value(match preview {
                Some(text) => purrdf_hash::error::ErrorValue::Text(text.as_ref()),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        },
        BadTag { kind, word } => "wkt-bad-tag" { "kind" => Code(kind.wkt_keyword()), "word" => Text(word.as_ref()) },
        BadTagFollower { tag, word } => "wkt-bad-tag-follower" { "tag" => Text(tag.as_ref()), "word" => Text(word.as_ref()) },
        EmptyList { production } => "wkt-empty-list" { "production" => Code(production) },
        ExtraOrdinate { dimension } => "wkt-extra-ordinate" { "dimension" => Record(dimension) },
    }
}

impl WktReason<&str> {
    fn into_owned<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<WktReason, StorageError> {
        Ok(match self {
            Self::TrailingText { found } => WktReason::TrailingText {
                found: memory.string(found)?,
            },
            Self::ExpectedByte {
                expected,
                production,
                found,
            } => WktReason::ExpectedByte {
                expected,
                production,
                found: memory.string(found)?,
            },
            Self::ExpectedNumber { found } => WktReason::ExpectedNumber {
                found: memory.string(found)?,
            },
            Self::InvalidNumber { lexical } => WktReason::InvalidNumber {
                lexical: memory.string(lexical)?,
            },
            Self::UnclosedCrs => WktReason::UnclosedCrs,
            Self::EmptyCrs => WktReason::EmptyCrs,
            Self::UnknownKeyword { word, preview } => WktReason::UnknownKeyword {
                word: memory.string(word)?,
                preview: preview.map(|text| memory.string(text)).transpose()?,
            },
            Self::BadTag { kind, word } => WktReason::BadTag {
                kind,
                word: memory.string(word)?,
            },
            Self::BadTagFollower { tag, word } => WktReason::BadTagFollower {
                tag: memory.string(tag)?,
                word: memory.string(word)?,
            },
            Self::EmptyList { production } => WktReason::EmptyList { production },
            Self::ExtraOrdinate { dimension } => WktReason::ExtraOrdinate { dimension },
        })
    }
}

/// Original coordinate position whose numeric token failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WktOrdinateContext {
    /// Original declared dimension.
    pub dimension: CoordDim,
    /// Original one-based ordinate index.
    pub index: usize,
}
impl purrdf_hash::error::StructuredError for WktOrdinateContext {
    purrdf_hash::error_visit! {
        @record self { dimension, index } => "wkt-ordinate-context" {
            "dimension" => Record(&self.dimension), "index" => Unsigned(self.index as u128),
            "expected" => Unsigned(self.dimension.ordinates() as u128)
        }
    }
}

/// Original WKT grammar refusal, without a pre-rendered message.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WktError {
    /// Original byte offset in the complete caller literal.
    pub position: usize,
    /// Original grammar decision and its source tokens.
    pub reason: WktReason,
    /// Original coordinate context, when a numeric token was being read.
    pub ordinate: Option<WktOrdinateContext>,
}
impl purrdf_hash::error::StructuredError for WktError {
    purrdf_hash::error_visit! {
        @record self { position, reason, ordinate } => "wkt-syntax" {
            "position" => Unsigned(self.position as u128), "reason" => Record(&self.reason),
            "ordinate" => Value(match &self.ordinate {
                Some(context) => purrdf_hash::error::ErrorValue::Record(context),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        }
    }
}
impl WktError {
    pub(crate) fn capture(
        position: usize,
        reason: WktReason<&str>,
        storage: Option<&crate::carrier::CarrierStorage<'_>>,
    ) -> super::GeoError {
        crate::carrier::with_storage(storage, |memory| {
            memory.try_scope(|memory| {
                let reason = reason.into_owned(memory)?;
                memory
                    .boxed(Self {
                        position,
                        reason,
                        ordinate: None,
                    })
                    .map(super::GeoError::WktSyntax)
            })
        })
        .unwrap_or_else(super::GeoError::Storage)
    }

    pub(crate) fn owned_bytes(&self) -> Result<usize, StorageError> {
        let text = match &self.reason {
            WktReason::TrailingText { found: text }
            | WktReason::ExpectedByte {
                expected: _,
                production: _,
                found: text,
            }
            | WktReason::ExpectedNumber { found: text }
            | WktReason::InvalidNumber { lexical: text }
            | WktReason::BadTag {
                kind: _,
                word: text,
            } => text.capacity(),
            WktReason::UnknownKeyword { word, preview } => word
                .capacity()
                .checked_add(preview.as_ref().map_or(0, String::capacity))
                .ok_or(StorageError::SizeOverflow)?,
            WktReason::BadTagFollower { tag, word } => tag
                .capacity()
                .checked_add(word.capacity())
                .ok_or(StorageError::SizeOverflow)?,
            WktReason::UnclosedCrs
            | WktReason::EmptyCrs
            | WktReason::EmptyList { production: _ }
            | WktReason::ExtraOrdinate { dimension: _ } => 0,
        };
        core::alloc::Layout::new::<Self>()
            .size()
            .checked_add(text)
            .ok_or(StorageError::SizeOverflow)
    }
}

impl core::fmt::Display for WktError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use crate::wkt::Preview;
        let at = self.position;
        match &self.reason {
            WktReason::TrailingText { found } => write!(f, "a wktLiteral holds exactly one geometry, but text follows it at byte {at}: {}", Preview(found))?,
            WktReason::ExpectedByte { expected, production, found } => write!(f, "expected {:?} in {production} at byte {at}, found {}", char::from(*expected), Preview(found))?,
            WktReason::ExpectedNumber { found } => write!(f, "expected a number at byte {at}, found {}", Preview(found))?,
            WktReason::InvalidNumber { lexical } => write!(f, "{lexical:?} at byte {at} is not an ordinate this reader can represent exactly; a WKT ordinate is an XSD decimal or double lexical form (so NaN and INF are not coordinates), with an exponent inside the exact parser's cap")?,
            WktReason::UnclosedCrs => write!(f, "the coordinate reference system prefix opened at byte {at} is never closed; it is written <IRI> before the geometry")?,
            WktReason::EmptyCrs => f.write_str("the coordinate reference system prefix is empty (<>); an empty IRI names no system, so a literal that means \"the default system\" omits the prefix entirely rather than writing an empty one")?,
            WktReason::UnknownKeyword { word, preview } => {
                let found = if word.is_empty() { preview.as_deref().unwrap_or("") } else { word.as_str() };
                write!(f, "expected a geometry keyword at byte {at}, found {}; WKT names seven (POINT, LINESTRING, POLYGON, MULTIPOINT, MULTILINESTRING, MULTIPOLYGON, GEOMETRYCOLLECTION) and writes any dimension tag as a separate token, so POINTZ is a keyword rather than POINT with a Z", Preview(found))?;
            }
            WktReason::BadTag { kind, word } => write!(f, "expected a dimension tag (Z, M or ZM), EMPTY, or '(' after {} at byte {at}, found {word:?}", kind.wkt_keyword())?,
            WktReason::BadTagFollower { tag, word } => write!(f, "expected EMPTY or '(' after the {tag:?} dimension tag at byte {at}, found {word:?}")?,
            WktReason::EmptyList { production } => write!(f, "{production} has an empty pair of parentheses at byte {at}; a geometry with no members is written EMPTY, and () is neither that nor a list")?,
            WktReason::ExtraOrdinate { dimension } => write!(f, "a {} position has {} ordinates, but another number follows the last one at byte {at}; the dimension is written once in the tag and governs every position", dimension.name(), dimension.ordinates())?,
        }
        if let Some(context) = &self.ordinate {
            write!(
                f,
                " — this is ordinate {} of the {} a {} position carries",
                context.index,
                context.dimension.ordinates(),
                context.dimension.name()
            )?;
        }
        Ok(())
    }
}
impl std::error::Error for WktError {}
