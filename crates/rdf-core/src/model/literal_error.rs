// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation-free literal component refusals and admitted diagnostic retention.

use purrdf_iri::langtag::LanguageTagError;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError, retain_native_cause};

/// Why a literal's datatype, language and base direction cannot be admitted.
/// The grammar cause owns no caller text; copying this error allocates nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LiteralComponentError {
    /// The original language-tag parser refused the offered tag.
    LanguageTag(LanguageTagError),
    /// The datatype differs from the one implied by language and direction.
    DatatypeMismatch,
    /// A base direction was offered without a language tag.
    DirectionWithoutLanguage,
    /// A language-string datatype was offered without a language tag.
    LanguageDatatypeWithoutLanguage,
}

impl LiteralComponentError {
    /// The existing grammar or literal-shape diagnostic identity.
    #[must_use]
    pub const fn diagnostic_code(self) -> &'static str {
        match self {
            Self::LanguageTag(cause) => cause.diagnostic_code(),
            Self::DatatypeMismatch
            | Self::DirectionWithoutLanguage
            | Self::LanguageDatatypeWithoutLanguage => super::LITERAL_SHAPE_CODE,
        }
    }

    /// The unchanged plain component refusal, without diagnostic prefixes.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::LanguageTag(cause) => cause.message(),
            Self::DatatypeMismatch => {
                "literal datatype does not match its language and base direction"
            }
            Self::DirectionWithoutLanguage => "a base direction requires a language tag",
            Self::LanguageDatatypeWithoutLanguage => {
                "a language-string datatype requires a language tag"
            }
        }
    }

    /// Retain this native cause and diagnostic through the original account.
    /// The shared cause has an independent Resident owner; the diagnostic's
    /// presentation and attachment remain on the supplied account.
    ///
    /// # Errors
    /// Returns the first retention, construction or admission refusal. Partial
    /// diagnostic storage dies before its grant is refunded on failure.
    pub fn into_diagnostic_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<crate::RdfDiagnostic, StorageError> {
        memory.scope(|memory| {
            let diagnostic = crate::RdfDiagnostic::try_error_with_memory(
                self.diagnostic_code(),
                self.message(),
                memory,
            )?;
            diagnostic.with_native_cause(retain_native_cause(self)?, memory)
        })
    }

    /// Retain this refusal for a resident native or host diagnostic.
    ///
    /// # Errors
    /// Returns the original physical construction or retention refusal.
    pub fn into_diagnostic(self) -> Result<crate::RdfDiagnostic, StorageError> {
        self.into_diagnostic_with_memory(&mut Memory::new(&mut Resident))
    }
}

impl core::fmt::Display for LiteralComponentError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.message())
    }
}

purrdf_hash::error_sources!(LiteralComponentError, LanguageTag);

impl purrdf_hash::error::StructuredError for LiteralComponentError {
    purrdf_hash::error_visit! {
        LanguageTag(cause) => "rdf-literal-component-language" { "cause" => Cause(cause) },
        DatatypeMismatch => "rdf-literal-component-datatype-mismatch" {},
        DirectionWithoutLanguage => "rdf-literal-component-direction-without-language" {},
        LanguageDatatypeWithoutLanguage => "rdf-literal-component-datatype-without-language" {},
    }
}
