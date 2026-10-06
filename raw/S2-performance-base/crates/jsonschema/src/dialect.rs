// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The supported dialects, their vocabularies and the keywords that hold
//! subschemas in each — all of it code, none of it read from a meta-schema.
//!
//! A dialect is named by the identifier of its published meta-schema. Those
//! identifiers are recognised here as specification identifiers; the
//! meta-schema *documents* are never part of this crate and must be
//! registered by the caller (see [`crate::Metaschemas`]) before anything that
//! needs them — meta-validation, or a `$ref` into one — can compile.

/// A JSON Schema dialect this crate evaluates.
///
/// Every schema resource is evaluated under exactly one dialect: the one its
/// `$schema` names (directly, or through a custom meta-schema built on it),
/// else the one of the resource enclosing it, else the one its document was
/// registered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Dialect {
    /// Draft-07 (`draft-handrews-json-schema-01`):
    /// `http://json-schema.org/draft-07/schema#`.
    Draft07,
    /// Draft 2019-09: `https://json-schema.org/draft/2019-09/schema`.
    Draft2019_09,
    /// Draft 2020-12: `https://json-schema.org/draft/2020-12/schema`.
    Draft2020_12,
}

impl Dialect {
    /// The identifier of the dialect's meta-schema, without a fragment.
    ///
    /// ```
    /// use purrdf_jsonschema::Dialect;
    ///
    /// assert_eq!(Dialect::Draft07.uri(), "http://json-schema.org/draft-07/schema");
    /// assert_eq!(Dialect::from_uri("http://json-schema.org/draft-07/schema#"), Some(Dialect::Draft07));
    /// assert_eq!(Dialect::from_uri("http://json-schema.org/draft-06/schema#"), None);
    /// ```
    pub const fn uri(self) -> &'static str {
        match self {
            Self::Draft07 => "http://json-schema.org/draft-07/schema",
            Self::Draft2019_09 => "https://json-schema.org/draft/2019-09/schema",
            Self::Draft2020_12 => "https://json-schema.org/draft/2020-12/schema",
        }
    }

    /// The dialect a `$schema` value names, with or without the empty
    /// fragment; `None` for anything else.
    pub fn from_uri(uri: &str) -> Option<Self> {
        match strip_empty_fragment(uri) {
            "http://json-schema.org/draft-07/schema" => Some(Self::Draft07),
            "https://json-schema.org/draft/2019-09/schema" => Some(Self::Draft2019_09),
            "https://json-schema.org/draft/2020-12/schema" => Some(Self::Draft2020_12),
            _ => None,
        }
    }

    /// The keywords whose values hold subschemas, for the identifier scan.
    pub(crate) const fn subschemas(self) -> &'static Subschemas {
        match self {
            Self::Draft07 => &DRAFT_07_SUBSCHEMAS,
            Self::Draft2019_09 => &DRAFT_2019_09_SUBSCHEMAS,
            Self::Draft2020_12 => &DRAFT_2020_12_SUBSCHEMAS,
        }
    }

    /// The vocabularies of the dialect's own meta-schema.
    pub(crate) const fn default_vocabularies(self) -> Vocabularies {
        match self {
            // Draft-07 has no vocabularies: every keyword it defines is in force.
            Self::Draft07 => {
                Vocabularies(Vocabulary::Applicator as u8 | Vocabulary::Validation as u8)
            }
            // 2019-09 keeps `unevaluated*` in the Applicator vocabulary.
            Self::Draft2019_09 | Self::Draft2020_12 => Vocabularies(
                Vocabulary::Applicator as u8
                    | Vocabulary::Unevaluated as u8
                    | Vocabulary::Validation as u8,
            ),
        }
    }

    /// What declaring `vocabulary` (required or not) in a custom
    /// meta-schema's `$vocabulary` turns on; `None` for a vocabulary this
    /// dialect does not define.
    pub(crate) fn vocabulary(self, vocabulary: &str, required: bool) -> Option<Vocabularies> {
        let none = Vocabularies::NONE;
        match self {
            Self::Draft07 => None,
            Self::Draft2019_09 => {
                let name =
                    vocabulary.strip_prefix("https://json-schema.org/draft/2019-09/vocab/")?;
                Some(match name {
                    "core" | "meta-data" | "content" => none,
                    "applicator" => none
                        .with(Vocabulary::Applicator)
                        .with(Vocabulary::Unevaluated),
                    "validation" => none.with(Vocabulary::Validation),
                    // Validation §7.2.1: `true` makes `format` an assertion.
                    "format" if required => none.with(Vocabulary::FormatRequired),
                    "format" => none,
                    _ => return None,
                })
            }
            Self::Draft2020_12 => {
                let name =
                    vocabulary.strip_prefix("https://json-schema.org/draft/2020-12/vocab/")?;
                Some(match name {
                    "core" | "format-annotation" | "meta-data" | "content" => none,
                    "applicator" => none.with(Vocabulary::Applicator),
                    "unevaluated" => none.with(Vocabulary::Unevaluated),
                    "validation" => none.with(Vocabulary::Validation),
                    "format-assertion" => none.with(Vocabulary::FormatAssertion),
                    _ => return None,
                })
            }
        }
    }

    /// The syntax of an anchor name: 2020-12 Core §8.2.2 `anchorString`, and
    /// the plain-name syntax of 2019-09 `$anchor` and draft-07 `$id`
    /// fragments (a letter, then letters, digits, `-`, `_`, `:` or `.`).
    pub(crate) fn is_anchor_name(self, name: &str) -> bool {
        let mut bytes = name.bytes();
        match self {
            Self::Draft2020_12 => {
                bytes
                    .next()
                    .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
                    && bytes.all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_')
                    })
            }
            Self::Draft07 | Self::Draft2019_09 => {
                bytes
                    .next()
                    .is_some_and(|first| first.is_ascii_alphabetic())
                    && bytes.all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b':')
                    })
            }
        }
    }
}

/// `uri` without a trailing empty fragment.
fn strip_empty_fragment(uri: &str) -> &str {
    uri.strip_suffix('#').unwrap_or(uri)
}

/// Published dialect identifiers this crate recognises and refuses: every
/// draft before draft-07, the unversioned "latest" identifiers, and the
/// unreleased successors of 2020-12.
const UNSUPPORTED: &[&str] = &[
    "http://json-schema.org/schema",
    "http://json-schema.org/draft-03/schema",
    "http://json-schema.org/draft-04/schema",
    "http://json-schema.org/draft-05/schema",
    "http://json-schema.org/draft-06/schema",
    "https://json-schema.org/schema",
    "https://json-schema.org/draft-03/schema",
    "https://json-schema.org/draft-04/schema",
    "https://json-schema.org/draft-05/schema",
    "https://json-schema.org/draft-06/schema",
    "https://json-schema.org/draft/next/schema",
    "https://json-schema.org/v1",
];

/// Whether a `$schema` value names a published dialect this crate refuses.
pub(crate) fn is_unsupported(uri: &str) -> bool {
    UNSUPPORTED.contains(&strip_empty_fragment(uri))
}

/// Whether `uri` (without fragment) is a published meta-schema of a
/// supported dialect: a dialect identifier or one of the 2019-09 and 2020-12
/// vocabulary meta-schemas. A reference to one that is not registered is a
/// missing meta-schema, not an arbitrary unresolved reference.
pub(crate) fn is_published_metaschema(uri: &str) -> bool {
    Dialect::from_uri(uri).is_some()
        || uri.starts_with("https://json-schema.org/draft/2019-09/meta/")
        || uri.starts_with("https://json-schema.org/draft/2020-12/meta/")
}

/// The keywords whose values hold subschemas in one dialect.
#[derive(Debug)]
pub(crate) struct Subschemas {
    /// Values that are objects of subschemas.
    pub(crate) maps: &'static [&'static str],
    /// Values that are arrays of subschemas (when they are arrays).
    pub(crate) arrays: &'static [&'static str],
    /// Values that are one subschema (when they are objects or booleans).
    pub(crate) singles: &'static [&'static str],
}

const DRAFT_07_SUBSCHEMAS: Subschemas = Subschemas {
    maps: &[
        "definitions",
        "properties",
        "patternProperties",
        "dependencies",
    ],
    arrays: &["allOf", "anyOf", "oneOf", "items"],
    singles: &[
        "additionalProperties",
        "propertyNames",
        "items",
        "additionalItems",
        "contains",
        "not",
        "if",
        "then",
        "else",
    ],
};

const DRAFT_2019_09_SUBSCHEMAS: Subschemas = Subschemas {
    maps: &[
        "$defs",
        "definitions",
        "properties",
        "patternProperties",
        "dependentSchemas",
        "dependencies",
    ],
    arrays: &["allOf", "anyOf", "oneOf", "items"],
    singles: &[
        "additionalProperties",
        "propertyNames",
        "items",
        "additionalItems",
        "contains",
        "not",
        "if",
        "then",
        "else",
        "unevaluatedItems",
        "unevaluatedProperties",
        "contentSchema",
    ],
};

const DRAFT_2020_12_SUBSCHEMAS: Subschemas = Subschemas {
    maps: &[
        "$defs",
        "definitions",
        "properties",
        "patternProperties",
        "dependentSchemas",
        "dependencies",
    ],
    arrays: &["allOf", "anyOf", "oneOf", "prefixItems"],
    singles: &[
        "additionalProperties",
        "propertyNames",
        "items",
        "contains",
        "not",
        "if",
        "then",
        "else",
        "unevaluatedItems",
        "unevaluatedProperties",
        "contentSchema",
    ],
};

/// A vocabulary (or vocabulary setting) whose keywords assert or apply. The
/// Core, Meta-Data, Format-Annotation and Content vocabularies need no flag:
/// their keywords are identifiers or annotations whichever vocabularies are
/// in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Vocabulary {
    Applicator = 1,
    Unevaluated = 2,
    Validation = 4,
    /// The 2020-12 Format-Assertion vocabulary: `format` asserts, and a
    /// format that cannot be checked is refused.
    FormatAssertion = 8,
    /// The 2019-09 Format vocabulary declared `true`: `format` asserts, and
    /// an unknown format stays an annotation.
    FormatRequired = 16,
}

/// Which vocabularies a schema resource is evaluated under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Vocabularies(u8);

impl Vocabularies {
    pub(crate) const NONE: Self = Self(0);

    pub(crate) const fn with(self, vocabulary: Vocabulary) -> Self {
        Self(self.0 | vocabulary as u8)
    }

    pub(crate) const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether `vocabulary` is in force.
    pub(crate) const fn has(self, vocabulary: Vocabulary) -> bool {
        self.0 & vocabulary as u8 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_round_trip_and_old_drafts_are_recognised_as_unsupported() {
        for dialect in [
            Dialect::Draft07,
            Dialect::Draft2019_09,
            Dialect::Draft2020_12,
        ] {
            assert_eq!(Dialect::from_uri(dialect.uri()), Some(dialect));
            assert_eq!(
                Dialect::from_uri(&format!("{}#", dialect.uri())),
                Some(dialect)
            );
            assert!(!is_unsupported(dialect.uri()));
        }
        assert!(is_unsupported("http://json-schema.org/draft-06/schema#"));
        assert!(is_unsupported("http://json-schema.org/draft-04/schema"));
        assert_eq!(
            Dialect::from_uri("http://json-schema.org/draft-06/schema#"),
            None
        );
    }

    #[test]
    fn anchor_syntax_follows_each_dialect() {
        assert!(Dialect::Draft2020_12.is_anchor_name("_a"));
        assert!(!Dialect::Draft2019_09.is_anchor_name("_a"));
        assert!(Dialect::Draft2019_09.is_anchor_name("a:b"));
        assert!(!Dialect::Draft2020_12.is_anchor_name("a:b"));
        assert!(Dialect::Draft07.is_anchor_name("foo"));
        assert!(!Dialect::Draft07.is_anchor_name("1foo"));
    }
}
