// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native vocabulary declarations and unit mismatches.

/// The namespace declaration that was empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VocabularyNamespace {
    /// GeoSPARQL geometry terms.
    Core,
    /// GeoSPARQL functions.
    Function,
    /// Simple Features geometry classes.
    SimpleFeatures,
}

impl purrdf_hash::error::StructuredError for VocabularyNamespace {
    purrdf_hash::error_visit! {
        Core => "vocabulary-namespace-core" {},
        Function => "vocabulary-namespace-function" {},
        SimpleFeatures => "vocabulary-namespace-simple-features" {},
    }
}

/// Original vocabulary inputs at a configuration or unit-domain refusal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VocabularyError {
    /// No exact conversion factor is registered for this output unit.
    UnregisteredOutputUnit {
        /// Original requested unit identifier.
        unit: String,
    },
    /// No unit was declared for this carrier.
    UndeclaredUnit {
        /// Original carrier identifier.
        crs: String,
    },
    /// A different unit was requested for a planar measurement.
    UnitMismatch {
        /// Original carrier identifier.
        crs: String,
        /// Declared unit.
        declared: String,
        /// Requested unit.
        requested: String,
    },
    /// Metric measurement has no declared metre identifier.
    MissingMetre {
        /// Original carrier identifier.
        crs: String,
    },
    /// Geometry-type output has no declared class namespace.
    MissingSimpleFeatures {
        /// Original geometry kind being projected.
        kind: crate::geom::GeometryKind,
    },
    /// A namespace declaration was empty.
    EmptyNamespace(VocabularyNamespace),
    /// A unit declaration was empty.
    EmptyUnit {
        /// Original carrier identifier.
        crs: String,
    },
    /// One carrier received different unit declarations.
    ConflictingUnit {
        /// Original carrier identifier.
        crs: String,
        /// Existing declared unit.
        first: String,
        /// Refused declared unit.
        second: String,
    },
    /// The metre identifier was empty.
    EmptyMetre,
}

impl purrdf_hash::error::StructuredError for VocabularyError {
    purrdf_hash::error_visit! {
        UnregisteredOutputUnit { unit } => "vocabulary-unregistered-output-unit" { "unit" => Iri(unit) },
        UndeclaredUnit { crs } => "vocabulary-undeclared-unit" { "crs" => Iri(crs) },
        UnitMismatch { crs, declared, requested } => "vocabulary-unit-mismatch" {
            "crs" => Iri(crs), "declared" => Iri(declared), "requested" => Iri(requested)
        },
        MissingMetre { crs } => "vocabulary-missing-metre" { "crs" => Iri(crs) },
        MissingSimpleFeatures { kind } => "vocabulary-missing-simple-features" { "kind" => Code(kind.geojson_type()) },
        EmptyNamespace(namespace) => "vocabulary-empty-namespace" { "namespace" => Record(namespace) },
        EmptyUnit { crs } => "vocabulary-empty-unit" { "crs" => Iri(crs) },
        ConflictingUnit { crs, first, second } => "vocabulary-conflicting-unit" {
            "crs" => Iri(crs), "first" => Iri(first), "second" => Iri(second)
        },
        EmptyMetre => "vocabulary-empty-metre" {},
    }
}

impl VocabularyError {
    pub(crate) fn unregistered_output_unit(unit: &crate::Crs) -> super::GeoError {
        let mut progress = crate::context::WorkProgress::integer(None);
        crate::context::Retained::produce(
            &mut progress,
            |memory| {
                memory
                    .string(unit.as_str())
                    .map(|unit| Self::UnregisteredOutputUnit { unit })
                    .map_err(super::GeoError::Storage)
            },
            |_, _| Ok(()),
        )
        .map_or_else(|error| error, super::GeoError::Vocabulary)
    }

    /// A unit mismatch belongs to the original row's expression-error channel.
    #[must_use]
    pub const fn is_expression_error(&self) -> bool {
        matches!(
            self,
            Self::UnitMismatch { .. } | Self::UnregisteredOutputUnit { .. }
        )
    }
}

impl core::fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnregisteredOutputUnit { unit } => write!(f, "unregistered linear output unit <{unit}>"),
            Self::UndeclaredUnit { crs } => write!(f,
                "no linear unit is declared for the coordinate reference system <{crs}>; a measurement cannot be reported in a unit purrdf-geo was never told, and PurRDF ships no coordinate-reference-system database to derive one from — declare it with GeoVocabBuilder::declare_crs_unit"),
            Self::UnitMismatch { crs, declared, requested } => write!(f,
                "a measurement of a geometry in <{crs}> is computed in that system's declared unit <{declared}>, and <{requested}> was requested; purrdf-geo converts no units, so it refuses rather than reporting the number under a unit it does not denote"),
            Self::MissingMetre { .. } => f.write_str("the metric GeoSPARQL functions report metres, and no IRI has been declared as the metre; PurRDF mints no vocabulary IRIs, so declare it with GeoVocabBuilder::declare_metre"),
            Self::MissingSimpleFeatures { .. } => f.write_str(crate::GeoVocab::MISSING_SIMPLE_FEATURES_NAMESPACE),
            Self::EmptyNamespace(namespace) => {
                let label = match namespace {
                    VocabularyNamespace::Core => "geo:",
                    VocabularyNamespace::Function => "geof:",
                    VocabularyNamespace::SimpleFeatures => return f.write_str("the Simple Features geometry-type namespace may not be empty"),
                };
                write!(f, "the {label} namespace may not be empty; PurRDF mints no vocabulary IRIs, so the GeoSPARQL namespaces must be supplied by the caller")
            },
            Self::EmptyUnit { .. } => f.write_str("a unit IRI may not be empty; PurRDF mints no vocabulary IRIs, so the unit of a coordinate reference system must be supplied by the caller"),
            Self::ConflictingUnit { crs, first, second } => write!(f,
                "<{crs}> is already declared to have the linear unit <{first}>, and <{second}> was declared for it as well; a silently replaced declaration would change every measurement's meaning with no symptom"),
            Self::EmptyMetre => f.write_str("the metre IRI may not be empty"),
        }
    }
}
impl std::error::Error for VocabularyError {}
