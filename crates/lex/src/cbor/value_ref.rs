// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared owned and source-backed CBOR inspection.

use super::decode::{BorrowedChildren, BorrowedEntries, BorrowedItem};
use super::{DecodeError, Value, head};
use std::borrow::Cow;

// Both cursor types preserve the underlying iterator's cardinality guarantees.
macro_rules! cursor_size_hint {
    () => {
        fn size_hint(&self) -> (usize, Option<usize>) {
            match self {
                Self::Owned(items) => items.size_hint(),
                Self::Encoded(items) => items.size_hint(),
            }
        }
    };
}

#[derive(Clone, Copy, Debug)]
/// A value borrowed from an owned tree or its validated source bytes.
pub enum ValueRef<'a> {
    /// An existing owned value, without a clone.
    Owned(&'a Value),
    /// A validated item borrowing its original byte owner.
    Encoded(BorrowedItem<'a>),
}

impl<'a> ValueRef<'a> {
    /// Borrow an owned value or decode only this source-backed item.
    ///
    /// # Errors
    /// Preserves the original CBOR inspection refusal.
    pub fn to_value(self) -> Result<Cow<'a, Value>, DecodeError> {
        match self {
            Self::Owned(value) => Ok(Cow::Borrowed(value)),
            Self::Encoded(value) => value.decode().map(Cow::Owned),
        }
    }

    /// Read an integer in the full CBOR signed value space.
    pub fn integer(self) -> Option<i128> {
        match self {
            Self::Owned(value) => value.as_integer().map(i128::from),
            Self::Encoded(value) => {
                let major = value.major();
                if major != head::UNSIGNED && major != head::NEGATIVE {
                    return None;
                }
                let mut bytes = value.encoded();
                let number = i128::from(
                    head::read_argument(&mut bytes, major)
                        .expect("the shared decoder validated this integer argument"),
                );
                Some(if major == head::UNSIGNED {
                    number
                } else {
                    -1 - number
                })
            }
        }
    }

    /// Borrow definite text or retain the decoder's concatenated chunks.
    ///
    /// # Errors
    /// Preserves the original CBOR inspection refusal.
    pub fn text(self) -> Result<Option<Cow<'a, str>>, DecodeError> {
        match self {
            Self::Owned(value) => Ok(value.as_text().map(Cow::Borrowed)),
            Self::Encoded(value) => value.text(),
        }
    }

    /// Borrow definite bytes or retain the decoder's concatenated chunks.
    ///
    /// # Errors
    /// Preserves the original CBOR inspection refusal.
    pub fn bytes(self) -> Result<Option<Cow<'a, [u8]>>, DecodeError> {
        match self {
            Self::Owned(value) => Ok(value.as_bytes().map(Cow::Borrowed)),
            Self::Encoded(value) => value.bytes(),
        }
    }

    /// Whether this value is a map.
    pub fn is_map(self) -> bool {
        match self {
            Self::Owned(value) => matches!(value, Value::Map(_)),
            Self::Encoded(value) => value.major() == head::MAP,
        }
    }

    /// The tag and its sole child, preserving the original tag number.
    pub fn tag(self) -> Option<(u64, Self)> {
        match self {
            Self::Owned(Value::Tag(tag, item)) => Some((*tag, Self::Owned(item))),
            Self::Encoded(value) => value.tag().map(|(tag, item)| (tag, Self::Encoded(item))),
            Self::Owned(_) => None,
        }
    }

    /// The array cardinality and values, if this is an array.
    ///
    /// # Errors
    /// Preserves the original CBOR inspection refusal.
    pub fn array(self) -> Result<Option<ArrayRef<'a>>, DecodeError> {
        match self {
            Self::Owned(Value::Array(items)) => Ok(Some(ArrayRef {
                len: items.len(),
                values: Values::Owned(items.iter()),
            })),
            Self::Encoded(value) => {
                let Some(items) = value.array() else {
                    return Ok(None);
                };
                let mut bytes = value.encoded();
                let len = match head::read_argument(&mut bytes, head::ARRAY) {
                    Some(len) => usize::try_from(len)
                        .expect("a validated array's children fit its source slice"),
                    None => items
                        .clone()
                        .try_fold(0usize, |count, item| item.map(|_| count + 1))?,
                };
                Ok(Some(ArrayRef {
                    len,
                    values: Values::Encoded(items),
                }))
            }
            Self::Owned(_) => Ok(None),
        }
    }

    /// The map cardinality and entries, if this is a map.
    ///
    /// # Errors
    /// Preserves the original CBOR inspection refusal.
    pub fn map(self) -> Result<Option<MapRef<'a>>, DecodeError> {
        match self {
            Self::Owned(Value::Map(entries)) => Ok(Some(MapRef {
                len: entries.len(),
                entries: Entries::Owned(entries.iter()),
            })),
            Self::Encoded(value) => {
                let Some(entries) = value.map() else {
                    return Ok(None);
                };
                let mut bytes = value.encoded();
                let len = match head::read_argument(&mut bytes, head::MAP) {
                    Some(len) => usize::try_from(len)
                        .expect("a validated map's entries fit its source slice"),
                    None => entries
                        .clone()
                        .try_fold(0usize, |count, entry| entry.map(|_| count + 1))?,
                };
                Ok(Some(MapRef {
                    len,
                    entries: Entries::Encoded(entries),
                }))
            }
            Self::Owned(_) => Ok(None),
        }
    }

    /// The first text-keyed map value, preserving first-match semantics.
    ///
    /// # Errors
    /// Preserves the original CBOR inspection refusal.
    pub fn get(self, name: &str) -> Result<Option<Self>, DecodeError> {
        match self {
            Self::Owned(Value::Map(entries)) => Ok(super::map_get(entries, name).map(Self::Owned)),
            Self::Owned(_) => Ok(None),
            Self::Encoded(value) => value.get(name).map(|value| value.map(Self::Encoded)),
        }
    }
}

#[derive(Clone, Debug)]
/// An array's exact cardinality and forward value cursor.
pub struct ArrayRef<'a> {
    len: usize,
    values: Values<'a>,
}

impl ArrayRef<'_> {
    /// The exact number of children.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether the container has no children.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<'a> IntoIterator for ArrayRef<'a> {
    type Item = Result<ValueRef<'a>, DecodeError>;
    type IntoIter = Values<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.values
    }
}

#[derive(Clone, Debug)]
/// A forward value cursor with the original CBOR refusal type.
pub enum Values<'a> {
    /// Values from an existing owned array.
    Owned(core::slice::Iter<'a, Value>),
    /// Values from validated source bytes.
    Encoded(BorrowedChildren<'a>),
}

impl<'a> Iterator for Values<'a> {
    type Item = Result<ValueRef<'a>, DecodeError>;
    cursor_size_hint!();
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Owned(items) => items.next().map(|item| Ok(ValueRef::Owned(item))),
            Self::Encoded(items) => items.next().map(|item| item.map(ValueRef::Encoded)),
        }
    }
}

#[derive(Clone, Debug)]
/// A map's exact cardinality and forward entry cursor.
pub struct MapRef<'a> {
    len: usize,
    entries: Entries<'a>,
}

impl MapRef<'_> {
    /// The exact number of children.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether the container has no children.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<'a> IntoIterator for MapRef<'a> {
    type Item = Result<(ValueRef<'a>, ValueRef<'a>), DecodeError>;
    type IntoIter = Entries<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries
    }
}

#[derive(Clone, Debug)]
/// A forward entry cursor retaining duplicate keys and their order.
pub enum Entries<'a> {
    /// Entries from an existing owned map.
    Owned(core::slice::Iter<'a, (Value, Value)>),
    /// Entries from validated source bytes.
    Encoded(BorrowedEntries<'a>),
}

impl<'a> Iterator for Entries<'a> {
    type Item = Result<(ValueRef<'a>, ValueRef<'a>), DecodeError>;
    cursor_size_hint!();
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Owned(entries) => entries
                .next()
                .map(|(key, value)| Ok((ValueRef::Owned(key), ValueRef::Owned(value)))),
            Self::Encoded(entries) => entries.next().map(|entry| {
                entry.map(|(key, value)| (ValueRef::Encoded(key), ValueRef::Encoded(value)))
            }),
        }
    }
}
