// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native page-validation causes, kept separate from provider message data.

use super::summary::{PageSummaryError, PageSummaryField};
use crate::ir::pack::bits::PackBitsError;
use crate::{GlobalTermId, RdfStoreCapabilities, TermId};

/// The sealed page counter that changed at materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCounter {
    /// Provider-reported resident byte charge.
    Bytes,
    /// Page-local dictionary population.
    Terms,
    /// Asserted quad population.
    Quads,
}

impl PageCounter {
    /// Stable counter name, shared by inspection and human presentation.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bytes => "byte charge",
            Self::Terms => "term count",
            Self::Quads => "quad count",
        }
    }
}

/// Why page data cannot satisfy its certified metadata.
///
/// Native validation retains its original counts, identifiers and causes. Only
/// [`Message`](Self::Message) holds text supplied as data by a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageDataError {
    /// A provider's original diagnostic text, with no inferred native structure.
    Message(String),
    /// Original failure qualified by its chronological stack source.
    Source {
        /// Source ordinal in the stack.
        source: u64,
        /// Page requested within that source.
        requested: super::PageId,
        /// Unmodified provider failure, including its own reported page.
        original: Box<super::PageFault>,
    },
    /// A subset provider translated the caller's dense page address.
    Remapped(Box<super::PageFault>),
    /// A reference provider cannot address the requested page ordinal.
    OutOfRange {
        /// Original requested ordinal.
        index: u64,
        /// Actual provider population.
        pages: u64,
        /// Whether the address belongs to a subset provider.
        subset: bool,
    },
    /// Materialization belongs to a different provider generation.
    GenerationChanged {
        /// Requested immutable generation.
        expected: super::PageGeneration,
        /// Observed immutable generation.
        actual: super::PageGeneration,
    },
    /// The materialized counter differs from its sealed value.
    CounterChanged {
        /// Which exact counter was checked.
        counter: PageCounter,
        /// Sealed value.
        expected: u64,
        /// Materialized value.
        actual: u64,
    },
    /// Materialized capabilities differ from those captured at sealing.
    CapabilitiesChanged {
        /// Sealed flags.
        expected: RdfStoreCapabilities,
        /// Materialized flags.
        actual: RdfStoreCapabilities,
    },
    /// A local dictionary slot no longer contains its certified global value.
    TermValueChanged {
        /// Original page-local slot.
        local: TermId,
        /// Certified global identity.
        global: GlobalTermId,
    },
    /// Page contents cannot be summarized honestly.
    Summary(PageSummaryError),
    /// The complete summary digest changed.
    SummaryMismatch {
        /// Original sealed digest.
        sealed: u64,
        /// Digest recomputed from materialized content.
        observed: u64,
        /// First changed field, when the diagnostic refinement was performed.
        field: Option<PageSummaryField>,
    },
    /// Statistical metadata could not be decoded or merged.
    Statistics(PackBitsError),
    /// The provider page count cannot address a local metadata allocation.
    MetadataCapacity {
        /// Original provider count, before local-width conversion.
        pages: u64,
    },
    /// A requested page does not address this provider.
    UnknownPage,
    /// A requested page does not address the composed stack.
    UnknownStackPage,
}

purrdf_lex::variant_from!(PageDataError {
    Message(String),
    Summary(PageSummaryError),
    Statistics(PackBitsError)
});
purrdf_lex::variant_from!(PageDataError { Message(&str) as ToString::to_string });

impl purrdf_hash::error::StructuredError for PageDataError {
    purrdf_hash::error_visit! {
        Message(message) => "page-data-message" { "message" => Message(message) },
        Source { source, requested, original } => "page-source-failure" {
            "source" => Unsigned(u128::from(*source)),
            "requested" => Unsigned(u128::from(requested.0)),
            "cause" => Cause(original.as_ref()),
        },
        Remapped(cause) => "page-remapped-failure" { "cause" => Cause(cause.as_ref()) },
        OutOfRange { index, pages, subset } => "page-index-out-of-range" {
            "index" => Unsigned(u128::from(*index)),
            "pages" => Unsigned(u128::from(*pages)),
            "subset" => Boolean(*subset),
        },
        GenerationChanged { expected, actual } => "page-generation-changed" {
            "expected" => Unsigned(u128::from(expected.0)),
            "actual" => Unsigned(u128::from(actual.0)),
        },
        CounterChanged { counter, expected, actual } => "page-counter-changed" {
            "counter" => Code(counter.label()),
            "expected" => Value(match counter {
                PageCounter::Bytes => purrdf_hash::error::ErrorValue::ByteCount(u128::from(*expected)),
                PageCounter::Terms | PageCounter::Quads => purrdf_hash::error::ErrorValue::Unsigned(u128::from(*expected)),
            }),
            "actual" => Value(match counter {
                PageCounter::Bytes => purrdf_hash::error::ErrorValue::ByteCount(u128::from(*actual)),
                PageCounter::Terms | PageCounter::Quads => purrdf_hash::error::ErrorValue::Unsigned(u128::from(*actual)),
            }),
        },
        CapabilitiesChanged { expected, actual } => "page-capabilities-changed" {
            "expected" => Record(expected),
            "actual" => Record(actual),
        },
        TermValueChanged { local, global } => "page-term-value-changed" {
            "local" => Unsigned(local.index() as u128),
            "global" => Unsigned(u128::from(global.index())),
        },
        Summary(cause) => "page-summary-invalid" { "cause" => Cause(cause) },
        SummaryMismatch { sealed, observed, field } => "page-summary-mismatch" {
            "sealed" => Unsigned(u128::from(*sealed)),
            "observed" => Unsigned(u128::from(*observed)),
            "field" => Value(field.map_or(purrdf_hash::error::ErrorValue::Absent,
                |field| purrdf_hash::error::ErrorValue::Code(field.label()))),
        },
        Statistics(cause) => "page-statistics-invalid" { "cause" => Cause(cause) },
        MetadataCapacity { pages } => "page-metadata-capacity" { "pages" => Unsigned(u128::from(*pages)) },
        UnknownPage => "page-unknown" {},
        UnknownStackPage => "stack-page-unknown" {},
    }
}

impl std::fmt::Display for PageDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::Remapped(error) => error.message.fmt(f),
            Self::OutOfRange {
                index,
                pages,
                subset,
            } => {
                if *subset {
                    f.write_str("subset ")?;
                }
                write!(f, "page index {index} out of range 0..{pages}")
            }
            Self::Source {
                source,
                requested,
                original,
            } => write!(
                f,
                "source {source}, original page {}: {}",
                requested.0, original.message
            ),
            Self::GenerationChanged { expected, actual } => write!(
                f,
                "page belongs to generation {actual}, but generation {expected} was requested"
            ),
            Self::CounterChanged {
                counter,
                expected,
                actual,
            } => write!(
                f,
                "{} changed from sealed {expected} to materialized {actual}",
                counter.label()
            ),
            Self::CapabilitiesChanged { .. } => {
                f.write_str("page capabilities changed after sealing")
            }
            Self::TermValueChanged { local, .. } => {
                write!(f, "term value changed at local index {}", local.index())
            }
            Self::Summary(error) => error.fmt(f),
            Self::SummaryMismatch {
                sealed,
                observed,
                field,
            } => {
                write!(
                    f,
                    "materialized content no longer matches the summary it was sealed with: the sealed summary digests to {sealed:#018x}, the materialized page to {observed:#018x}"
                )?;
                if let Some(field) = field {
                    write!(f, " (the summaries disagree first at `{}`)", field.label())?;
                }
                Ok(())
            }
            Self::Statistics(error) => error.fmt(f),
            Self::MetadataCapacity { .. } => {
                f.write_str("page metadata does not fit a local allocation")
            }
            Self::UnknownPage => f.write_str("page does not address a local provider buffer"),
            Self::UnknownStackPage => f.write_str("unknown stack page"),
        }
    }
}

impl std::error::Error for PageDataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source { original, .. } => Some(original.as_ref()),
            Self::Remapped(original) => Some(original.as_ref()),
            Self::Summary(error) => Some(error),
            Self::Statistics(error) => Some(error),
            _ => None,
        }
    }
}

impl super::PageSlot {
    /// Shared metadata checks precede all content admission on both read paths.
    pub(super) fn validate_metadata(
        &self,
        page: &super::PageMaterialization,
    ) -> Result<(), PageDataError> {
        for (counter, expected, actual) in [
            (PageCounter::Bytes, self.byte_len, page.byte_len),
            (
                PageCounter::Terms,
                self.translation.term_count() as u64,
                page.dataset.term_count() as u64,
            ),
            (
                PageCounter::Quads,
                self.quad_count,
                page.dataset.quad_count() as u64,
            ),
        ] {
            if expected != actual {
                return Err(PageDataError::CounterChanged {
                    counter,
                    expected,
                    actual,
                });
            }
        }
        let actual = page.dataset.capabilities();
        if actual != self.caps {
            return Err(PageDataError::CapabilitiesChanged {
                expected: self.caps,
                actual,
            });
        }
        Ok(())
    }
}
