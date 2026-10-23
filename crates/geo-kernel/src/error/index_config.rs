// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original index-construction operands, before diagnostic presentation.

use crate::index::GeoIndexConfig;
use purrdf_core::TermValue;

/// Why an explicitly configured geographic projection cannot be constructed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IndexConfigError {
    /// There is no caller-selected serialization property.
    NoSerializations,
    /// A serialization property cannot occupy an RDF predicate position.
    Property {
        /// Original position in the caller's unsorted list.
        position: usize,
        /// The original refused term, moved without rendering or cloning.
        property: TermValue,
    },
    /// A named graph selector is not an IRI.
    Graph {
        /// Original graph-selector term.
        name: TermValue,
    },
    /// The serialization list contains the same property twice.
    RepeatedProperty {
        /// Original repeated property.
        property: TermValue,
    },
}

impl purrdf_hash::error::StructuredError for IndexConfigError {
    purrdf_hash::error_visit! {
        NoSerializations => "geo-index-no-serializations" {},
        Property { position, property } => "geo-index-non-iri-property" {
            "position" => Unsigned(*position as u128), "property" => Term(property)
        },
        Graph { name } => "geo-index-non-iri-graph" { "name" => Term(name) },
        RepeatedProperty { property } => "geo-index-repeated-property" { "property" => Term(property) },
    }
}

impl core::fmt::Display for IndexConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoSerializations => f.write_str("no serialization properties supplied; PurRDF mints no vocabulary, so there is no default serialization property set to fall back on — name the geo:as* properties your conformance class puts in play"),
            Self::Property { property, .. } => write!(f, "serialization property {property:?} is not an IRI; only an IRI can occupy the predicate position of an RDF statement"),
            Self::Graph { name } => write!(f, "named graph selector {name:?} is not an IRI"),
            Self::RepeatedProperty { property } => write!(f, "serialization property {property:?} is listed more than once; a repeat is a caller mistake, and silently deduplicating it would hide the mistake"),
        }
    }
}
impl std::error::Error for IndexConfigError {}

/// The original configured property and its nonliteral serialization object.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexSerializationError {
    /// Original serialization property, including every term component.
    pub property: TermValue,
    /// Original refused object, including blank scope and nested terms.
    pub object: TermValue,
    owned_bytes: usize,
}

impl purrdf_hash::error::StructuredError for IndexSerializationError {
    purrdf_hash::error_visit! {
        @record self { property, object, owned_bytes } => "geo-index-serialization-object" {
            "property" => Term(&self.property), "object" => Term(&self.object),
            "owned_bytes" => ByteCount(self.owned_bytes as u128)
        }
    }
}

impl IndexSerializationError {
    pub(crate) const fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }

    pub(crate) fn capture<S: purrdf_lex::allocation::Admission + ?Sized>(
        property: &TermValue,
        object: &TermValue,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
        mut before: impl FnMut(&TermValue) -> Result<(), super::GeoError>,
    ) -> Result<super::GeoError, super::GeoError> {
        memory.try_scope(|memory| {
            let baseline = memory.admitted_bytes();
            let mut copy = |value: &TermValue| {
                value
                    .try_clone_with_memory(memory, &mut before)
                    .map_err(|error| match error {
                        purrdf_core::OwnedTermFoldError::Visitor(error) => error,
                        purrdf_core::OwnedTermFoldError::Storage(error) => {
                            super::GeoError::Storage(error)
                        }
                    })
            };
            let property = copy(property)?;
            let object = copy(object)?;
            let mut cause = memory
                .boxed(Self {
                    property,
                    object,
                    owned_bytes: 0,
                })
                .map_err(super::GeoError::Storage)?;
            cause.owned_bytes =
                memory
                    .admitted_bytes()
                    .checked_sub(baseline)
                    .ok_or(super::GeoError::Storage(
                        purrdf_lex::allocation::StorageError::SizeOverflow,
                    ))?;
            Ok(super::GeoError::IndexSerialization(cause))
        })
    }
}

impl core::fmt::Display for IndexSerializationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "the object of the configured serialization property {:?} is {:?}, which is not a literal; only a literal can carry a geometry serialization, so this row names no geometry and passing over it would drop an answer the configuration asked for",
            self.property, self.object
        )
    }
}
impl std::error::Error for IndexSerializationError {}

/// Original index/view binding comparisons and lookup invariant.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IndexBindingError {
    /// The supplied configuration is not the index's original configuration.
    Configuration {
        /// Original index configuration.
        expected: GeoIndexConfig,
        /// Original caller-supplied configuration.
        actual: GeoIndexConfig,
    },
    /// The complete rebuilt projection has another source fingerprint.
    Source {
        /// Original index source fingerprint.
        expected: u64,
        /// Original supplied source fingerprint.
        actual: u64,
    },
    /// A view supplied an identifier that does not resolve in that view.
    ForeignTermId,
}

impl purrdf_hash::error::StructuredError for IndexBindingError {
    purrdf_hash::error_visit! {
        Configuration { expected, actual } => "geo-index-binding-configuration" { "expected" => Record(expected), "actual" => Record(actual) },
        Source { expected, actual } => "geo-index-binding-source" { "expected" => Unsigned(u128::from(*expected)), "actual" => Unsigned(u128::from(*actual)) },
        ForeignTermId => "geo-index-foreign-term-id" {},
    }
}

impl IndexBindingError {
    pub(crate) fn configuration(
        expected: &GeoIndexConfig,
        actual: &GeoIndexConfig,
    ) -> super::GeoError {
        let mut progress = crate::context::WorkProgress::integer(None);
        crate::context::Retained::produce(
            &mut progress,
            |memory| {
                memory.try_scope(|memory| {
                    Ok(Self::Configuration {
                        expected: expected.copy_with_memory(memory)?,
                        actual: actual.copy_with_memory(memory)?,
                    })
                })
            },
            |_, _| Ok(()),
        )
        .map_or_else(|error| error, super::GeoError::IndexBinding)
    }
}

impl core::fmt::Display for IndexBindingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Configuration { expected: _, actual: _ } => f.write_str("the configuration supplied to verify_binding is not the one this index was built under; the two would project different rows, so the comparison would answer a different question than the one asked"),
            Self::Source { expected, actual } => write!(f, "this GeoSPARQL index was built over a different dataset: its source digest is {expected:#018x} and the supplied dataset digests to {actual:#018x}. Rebuild the index from the dataset the query runs against, or pair the query with the dataset the index was built from — an index joined to the wrong dataset returns no rows and reports nothing"),
            Self::ForeignTermId => f.write_str("the dataset view is inconsistent: a term id does not name a term"),
        }
    }
}
impl std::error::Error for IndexBindingError {}
