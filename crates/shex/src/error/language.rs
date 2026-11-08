// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original ShExJ language-member positions and their fixed admission policy.

use core::fmt;
use purrdf_iri::langtag::Profile;

/// The native profile shared by ShExJ language admission and the ShExC lexer.
pub const SHEXJ_LANGUAGE_PROFILE: Profile = crate::lexer::LANGTAG_PROFILE;

/// The structured name of this family's fixed language admission policy.
pub const SHEXJ_LANGUAGE_POLICY: &str = "concrete-syntax-langtag-bounded";

// Changing the shared profile also requires an explicit policy-metadata decision.
const _: () = assert!(matches!(
    SHEXJ_LANGUAGE_PROFILE,
    Profile::ConcreteSyntaxLangtagBounded
));

/// The original ShExJ member whose language value failed admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageMember {
    /// An object literal's complete `language` value.
    ObjectLiteralLanguage,
    /// A `Language` value's complete `languageTag`.
    LanguageTag,
    /// A `LanguageStem` value's `stem`.
    LanguageStem,
    /// A `LanguageStemRange` value's string `stem`.
    RangeStem,
    /// A complete language tag in a range's string exclusion.
    RangeLanguageExclusion,
    /// A `LanguageStem` object in a range's exclusion array.
    RangeStemExclusion,
}

impl purrdf_hash::error::StructuredError for LanguageMember {
    purrdf_hash::error_visit! {
        ObjectLiteralLanguage => "shexj-object-literal-language" {},
        LanguageTag => "shexj-language-tag" {},
        LanguageStem => "shexj-language-stem" {},
        RangeStem => "shexj-language-range-stem" {},
        RangeLanguageExclusion => "shexj-language-range-exclusion" {},
        RangeStemExclusion => "shexj-language-range-stem-exclusion" {},
    }
}

impl fmt::Display for LanguageMember {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ObjectLiteralLanguage => "ObjectLiteral: \"language\"",
            Self::LanguageTag => "Language: \"languageTag\"",
            Self::LanguageStem => "LanguageStem: \"stem\"",
            Self::RangeStem => "LanguageStemRange: \"stem\"",
            Self::RangeLanguageExclusion => "LanguageStemRange exclusion",
            Self::RangeStemExclusion => "LanguageStemRange exclusion: \"stem\"",
        })
    }
}
