// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-authored and originally admitted native lexical text.

use core::{
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
};
use std::sync::Arc;

use super::SharedText;

/// Immutable spelling. Native clones retain the original text admission;
/// ordinary caller construction retains its caller-owned shared spelling.
#[derive(Clone)]
pub struct OwnedText(TextStorage);

#[derive(Clone)]
enum TextStorage {
    Caller(Arc<str>),
    Native(SharedText),
}

impl Deref for OwnedText {
    type Target = str;
    fn deref(&self) -> &str {
        match &self.0 {
            TextStorage::Caller(value) => value,
            TextStorage::Native(value) => value,
        }
    }
}
impl AsRef<str> for OwnedText {
    fn as_ref(&self) -> &str {
        self
    }
}
// Caller spelling conversion has one body; the typed From doors consume an
// owned String/Arc or copy a borrowed slice through its standard Arc conversion.
macro_rules! caller_text_from {
    ($($source:ty),+ $(,)?) => { $(
        impl From<$source> for OwnedText {
            fn from(value: $source) -> Self {
                Self(TextStorage::Caller(value.into()))
            }
        }
    )+ };
}
caller_text_from!(String, &str, Arc<str>);

impl From<SharedText> for OwnedText {
    fn from(value: SharedText) -> Self {
        Self(TextStorage::Native(value))
    }
}
impl PartialEq for OwnedText {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl Eq for OwnedText {}
impl PartialOrd for OwnedText {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OwnedText {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (**self).cmp(&**other)
    }
}
impl Hash for OwnedText {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}
impl fmt::Debug for OwnedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl fmt::Display for OwnedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}
