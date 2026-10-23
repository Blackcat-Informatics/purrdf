// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original coordinate-reference identifiers and native RFC 3987 causes.

/// Why an original coordinate-reference identifier was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrsError {
    /// The original identifier was empty.
    Empty,
    /// The original identifier was not a valid absolute IRI.
    Iri {
        /// Original identifier supplied by the caller or carrier prefix.
        iri: String,
        /// Original lexical or non-absolute-reference cause.
        cause: purrdf_iri::IriError,
    },
}

impl core::hash::Hash for CrsError {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // Native cause equality remains exact. Equal refusals necessarily
        // share their original identifier; no cause rendering is needed to
        // satisfy that hash contract or impose Hash on the IRI error API.
        core::hash::Hash::hash(&core::mem::discriminant(self), state);
        match self {
            Self::Empty => {}
            Self::Iri { iri, cause: _ } => core::hash::Hash::hash(iri, state),
        }
    }
}

impl purrdf_hash::error::StructuredError for CrsError {
    purrdf_hash::error_visit! {
        Empty => "crs-empty-identifier" {},
        Iri { iri, cause } => "crs-invalid-identifier" { "iri" => Iri(iri), "cause" => Cause(cause) },
    }
}

impl CrsError {
    pub(crate) fn owned_bytes(&self) -> Result<usize, purrdf_lex::allocation::StorageError> {
        let text = match self {
            Self::Empty => 0,
            Self::Iri { iri, cause } => iri
                .capacity()
                .checked_add(cause.owned_text_bytes())
                .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?,
        };
        core::alloc::Layout::new::<Self>()
            .size()
            .checked_add(text)
            .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)
    }
}

impl core::fmt::Display for CrsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => f.write_str("a coordinate reference system IRI may not be empty; PurRDF mints no vocabulary IRIs, so the CRS a geometry literal omits must be supplied by the caller's GeoVocab"),
            Self::Iri { iri, cause } => write!(f,
                "a coordinate reference system must be named by an ABSOLUTE IRI, and {iri:?} is not one ({cause}); OGC 22-047r1 requires the wktLiteral prefix to be \"a valid absolute IRI, as defined in [RFC 3987], enclosed in angled brackets\". Storing an invalid one verbatim would both fail to match an identical-looking declared system — reporting a unit the caller did declare as undeclared — and render a wktLiteral no conformant consumer can read"),
        }
    }
}
purrdf_lex::error_source!(CrsError => { Self::Iri { cause, .. } => cause });
