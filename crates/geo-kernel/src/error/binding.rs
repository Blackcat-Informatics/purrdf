// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original carrier-resolution configuration failures.

use crate::operation::OperationReference;
use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::StorageError;

/// Which linear-unit registration invariant failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitFactorReason {
    /// A scale must be strictly positive.
    NonPositive,
    /// The standard metre has exactly unit scale.
    OfficialMetre,
    /// An existing registration has a different scale.
    ConflictingRegistration,
}

impl purrdf_hash::error::StructuredError for UnitFactorReason {
    purrdf_hash::error_visit! {
        NonPositive => "unit-factor-nonpositive" {},
        OfficialMetre => "unit-factor-official-metre" {},
        ConflictingRegistration => "unit-factor-conflicting-registration" {},
    }
}

/// Which reference declaration a registration contradicted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceRegistrationReason {
    /// CRS84 fixes WGS84 with longitude/latitude axes.
    Crs84,
    /// EPSG:4326 fixes WGS84 with latitude/longitude axes.
    Epsg4326,
    /// The caller already registered a different reference.
    ConflictingRegistration,
    /// An image edge targets a reference different from its metric context.
    ImageEdgeContext,
    /// A transformed geometry targets a reference different from its context.
    TransformedContext,
}

impl purrdf_hash::error::StructuredError for ReferenceRegistrationReason {
    purrdf_hash::error_visit! {
        Crs84 => "reference-registration-crs84" {},
        Epsg4326 => "reference-registration-epsg4326" {},
        ConflictingRegistration => "reference-registration-conflicting" {},
        ImageEdgeContext => "reference-image-edge-context" {},
        TransformedContext => "reference-transformed-context" {},
    }
}

/// Which prepared-reference comparison failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceMismatchReason {
    /// A geographic index worker must use the prepared target reference.
    GeographicIndex,
    /// Geographic topology must use one actual prepared reference.
    GeographicTopology,
    /// Buffer materialization must use its original reference.
    BufferMaterialization,
    /// A prepared geodesic must match its metric context.
    GeodesicContext,
    /// A prepared geodesic must match its worker context.
    GeodesicWorker,
    /// A prepared point must match its worker context.
    PointContext,
    /// A newly prepared reference must match the worker context.
    PreparedReferenceWorker,
    /// A cached arc must match its context reference.
    ArcContext,
    /// An admitted geodesic binding must match the context binding.
    AdmittedGeodesic,
    /// A retained point binding must match its geodesic.
    GeodesicPoint,
    /// A retained endpoint binding must match its geodesic.
    GeodesicEndpoint,
    /// An image curve must use its actual context reference.
    ImageCurveContext,
    /// A symbolic point must use its actual target context.
    SymbolicPointContext,
}
impl purrdf_hash::error::StructuredError for ReferenceMismatchReason {
    purrdf_hash::error_visit! {
        GeographicIndex => "reference-mismatch-geographic-index" {},
        GeographicTopology => "reference-mismatch-geographic-topology" {},
        BufferMaterialization => "reference-mismatch-buffer-materialization" {},
        GeodesicContext => "reference-mismatch-geodesic-context" {},
        GeodesicWorker => "reference-mismatch-geodesic-worker" {},
        PointContext => "reference-mismatch-point-context" {},
        PreparedReferenceWorker => "reference-mismatch-prepared-reference-worker" {},
        ArcContext => "reference-mismatch-arc-context" {},
        AdmittedGeodesic => "reference-mismatch-admitted-geodesic" {},
        GeodesicPoint => "reference-mismatch-geodesic-point" {},
        GeodesicEndpoint => "reference-mismatch-geodesic-endpoint" {},
        ImageCurveContext => "reference-mismatch-image-curve-context" {},
        SymbolicPointContext => "reference-mismatch-symbolic-point-context" {},
    }
}
impl core::fmt::Display for ReferenceMismatchReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::GeographicIndex => {
                "geographic index query requires its actual prepared target reference"
            }
            Self::GeographicTopology => {
                "geographic topology requires one actual prepared reference"
            }
            Self::BufferMaterialization => {
                "buffer materialization requires its original actual geographic reference"
            }
            Self::GeodesicContext => "prepared geodesic and metric context references differ",
            Self::GeodesicWorker => "prepared geodesic and worker references differ",
            Self::PointContext => "prepared point and context references differ",
            Self::PreparedReferenceWorker => "prepared reference and worker context differ",
            Self::ArcContext => "arc and context references differ",
            Self::AdmittedGeodesic => "admitted geodesic and metric context references differ",
            Self::GeodesicPoint => "prepared point and geodesic references differ",
            Self::GeodesicEndpoint => "prepared endpoint and geodesic references differ",
            Self::ImageCurveContext => "image curve/context reference mismatch",
            Self::SymbolicPointContext => "symbolic point/context target reference mismatch",
        })
    }
}

/// Which image endpoint requirement failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageEndpointReason {
    /// The selected chain must realize the registered endpoints.
    Chain,
    /// Metric preparation also requires a geographic target in degrees.
    GeographicPreparation,
}

impl purrdf_hash::error::StructuredError for ImageEndpointReason {
    purrdf_hash::error_visit! {
        Chain => "image-endpoints-chain" {},
        GeographicPreparation => "image-endpoints-geographic-preparation" {},
    }
}

/// Which original image-target requirement failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageTargetReason {
    /// The requested physical metric contradicts the actual target unit.
    MetricUnits,
    /// The requested geographic metric contradicts the actual target realization or axes.
    MetricReference,
    /// A geographic image curve requires its actual declared target.
    CurveReference,
}
impl purrdf_hash::error::StructuredError for ImageTargetReason {
    purrdf_hash::error_visit! {
        MetricUnits => "image-target-metric-units" {},
        MetricReference => "image-target-metric-reference" {},
        CurveReference => "image-target-curve-reference" {},
    }
}

/// Which original geographic-index preparation comparison failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeoIndexAdmissionReason {
    /// The selected target declaration differs from the worker's actual reference.
    Reference,
    /// The original profile and worker use different admission policies.
    Policy,
}
impl purrdf_hash::error::StructuredError for GeoIndexAdmissionReason {
    purrdf_hash::error_visit! {
        Reference => "geographic-index-admission-reference" {},
        Policy => "geographic-index-admission-policy" {},
    }
}

/// Why explicit reference and operation bindings are inconsistent.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BindingConfigError {
    /// The original geographic-index target or preparation policy disagrees.
    GeographicIndexAdmission {
        /// First comparison that failed.
        reason: GeoIndexAdmissionReason,
        /// Original selected target identifier.
        target: crate::Crs,
        /// Original reference declared for that target.
        declared: crate::GeographicReference,
        /// Original worker reference.
        actual: crate::GeographicReference,
        /// Original profile policy.
        expected_policy: crate::ExecutionPolicy,
        /// Original worker policy.
        actual_policy: crate::ExecutionPolicy,
    },
    /// Automatic inversion requires an explicit certified domain for this model.
    InverseDomain {
        /// Original compiled operation identity.
        operation: Digest32,
        /// Original declared source endpoint.
        source: OperationReference,
        /// Original declared target endpoint.
        target: OperationReference,
        /// Original model's semantic descriptor.
        law: &'static [u8],
    },
    /// The original applicability rectangle has a nonpositive extent.
    Applicability {
        /// First coordinate axis whose extent failed.
        axis: crate::measure::Ordinate,
        /// Original first-axis lower bound.
        west: crate::context::Retained<crate::Rat>,
        /// Original first-axis upper bound.
        east: crate::context::Retained<crate::Rat>,
        /// Original second-axis lower bound.
        south: crate::context::Retained<crate::Rat>,
        /// Original second-axis upper bound.
        north: crate::context::Retained<crate::Rat>,
    },
    /// The original operation endpoints disagree with the model's units.
    OperationUnits {
        /// Original declared source endpoint.
        source: OperationReference,
        /// Original declared target endpoint.
        target: OperationReference,
        /// Original model's input unit.
        input: crate::operation::CoordinateUnit,
        /// Original model's output unit.
        output: crate::operation::CoordinateUnit,
        /// Original semantic model descriptor from the model's existing home.
        law: &'static [u8],
    },
    /// The original image metric and declared operation target disagree.
    ImageTarget {
        /// First rejected target requirement.
        reason: ImageTargetReason,
        /// Original selected operation target.
        target: OperationReference,
        /// Original requested physical metric, including its complete reference.
        metric: crate::operation::ImageMetric,
    },
    /// The original ordered operation chain has no component.
    EmptyOperationChain {
        /// Original number of supplied operations.
        operation_count: usize,
        /// Whether the empty chain was observed after compilation.
        compiled: bool,
    },
    /// The first unequal adjacent reference boundary in an operation chain.
    ChainBoundary {
        /// Zero-based index of the preceding operation.
        index: usize,
        /// Original preceding operation identity.
        preceding_operation: Digest32,
        /// Original following operation identity.
        following_operation: Digest32,
        /// Original target of the preceding operation.
        target: OperationReference,
        /// Original source of the following operation.
        source: OperationReference,
    },
    /// Source admission was prepared for a different execution policy.
    PreparationPolicy {
        /// Original source-admission policy.
        admitted: crate::ExecutionPolicy,
        /// Original query profile policy.
        expected: crate::ExecutionPolicy,
    },
    /// A linear-unit scale violates a declaration or positivity requirement.
    UnitFactor {
        /// Original unit identifier.
        unit: String,
        /// Exact refused factor under its original native owner.
        actual: crate::context::Retained<crate::Rat>,
        /// Original registered or standard factor, when one exists.
        expected: Option<crate::context::Retained<crate::Rat>>,
        /// Invariant checked by the registration path.
        reason: UnitFactorReason,
    },
    /// Metric reference selection received no source carriers.
    EmptyMetricSources,
    /// Two reachable carriers denote distinct physical metric surfaces.
    AmbiguousMetricSurface {
        /// First reachable carrier.
        first: String,
        /// Conflicting reachable carrier.
        second: String,
    },
    /// A selected chain disagrees with the registered image carriers.
    ImageEndpoints {
        /// Endpoint requirement checked by the caller.
        reason: ImageEndpointReason,
        /// Registered source endpoint.
        expected_source: OperationReference,
        /// Selected chain's source endpoint.
        actual_source: OperationReference,
        /// Registered target endpoint.
        expected_target: OperationReference,
        /// Selected chain's target endpoint.
        actual_target: OperationReference,
    },
    /// One operation identifier has incompatible registrations.
    ConflictingOperation {
        /// Original operation identifier.
        operation: String,
        /// Existing source carrier.
        first_source: String,
        /// Existing target carrier.
        first_target: String,
        /// Existing chain identity.
        first_chain: Digest32,
        /// Refused source carrier.
        second_source: String,
        /// Refused target carrier.
        second_target: String,
        /// Refused chain identity.
        second_chain: Digest32,
    },
    /// A named coordinate operation has no registration.
    UnknownOperation {
        /// Original operation identifier.
        operation: String,
    },
    /// A reference identifier has incompatible registrations.
    ReferenceRegistration {
        /// Original reference identifier.
        reference: String,
        /// Original refused reference declaration.
        actual: crate::GeographicReference,
        /// Original standard or previous declaration.
        expected: crate::GeographicReference,
        /// Declaration that the registration contradicts.
        reason: ReferenceRegistrationReason,
    },
    /// An operation's actual reference differs from its immutable preparation.
    ReferenceMismatch {
        /// Original failed comparison.
        reason: ReferenceMismatchReason,
        /// Original conflicting reference declaration.
        actual: crate::GeographicReference,
        /// Original required reference declaration.
        expected: crate::GeographicReference,
    },
    /// An opaque prepared binding does not match the required binding identity.
    ReferenceIdentity {
        /// Original failed comparison.
        reason: ReferenceMismatchReason,
        /// Original supplied binding identity.
        actual: crate::GeoBindingId,
        /// Original required binding identity.
        expected: crate::GeoBindingId,
    },
    /// A cached arc disagrees with the original numerical reference admission.
    CachedArc {
        /// Original worker reference.
        actual_reference: crate::GeographicReference,
        /// Original prepared geodesic reference.
        expected_reference: crate::GeographicReference,
        /// Original required binding identity.
        expected: crate::GeoBindingId,
        /// Original starting point binding.
        start: crate::GeoBindingId,
        /// Original ending point binding.
        end: crate::GeoBindingId,
        /// Original inverse solution binding.
        inverse: crate::GeoBindingId,
    },
    /// The reference is outside the two original native preparations.
    NonNativeReference {
        /// Original refused declaration.
        actual: crate::GeographicReference,
    },
    /// An operation endpoint does not realize its declared geographic carrier.
    InvalidEndpoint {
        /// Original carrier identifier.
        carrier: String,
        /// Required endpoint realization, units and axes.
        expected: OperationReference,
        /// Actual endpoint.
        actual: OperationReference,
    },
    /// Multiple distinct chains realize the requested source/target pair.
    MultipleOperations {
        /// Original source carrier.
        source: String,
        /// Original target carrier.
        target: String,
        /// First original chain identity.
        first: Digest32,
        /// Conflicting original chain identity.
        second: Digest32,
    },
    /// A carrier is declared with incompatible operation endpoints.
    ConflictingEndpoint {
        /// Original carrier identifier.
        carrier: String,
        /// First declaration.
        first: OperationReference,
        /// Conflicting declaration.
        second: OperationReference,
    },
}

impl purrdf_hash::error::StructuredError for BindingConfigError {
    purrdf_hash::error_visit! {
        GeographicIndexAdmission { reason, target, declared, actual, expected_policy, actual_policy } => "carrier-geographic-index-admission" {
            "reason" => Record(reason), "target" => Record(target), "declared" => Record(declared), "actual" => Record(actual),
            "expected_policy" => Record(expected_policy), "actual_policy" => Record(actual_policy)
        },
        InverseDomain { operation, source, target, law } => "carrier-operation-inverse-domain" {
            "operation" => Record(operation), "source" => Record(source), "target" => Record(target), "law" => Bytes(law)
        },
        Applicability { axis, west, east, south, north } => "carrier-operation-applicability" {
            "axis" => Record(axis), "west" => Record(&**west), "east" => Record(&**east),
            "south" => Record(&**south), "north" => Record(&**north)
        },
        OperationUnits { source, target, input, output, law } => "carrier-operation-units" {
            "source" => Record(source), "target" => Record(target), "input" => Record(input), "output" => Record(output), "law" => Bytes(law)
        },
        ImageTarget { reason, target, metric } => "carrier-image-target" {
            "reason" => Record(reason), "target" => Record(target), "metric" => Record(metric)
        },
        EmptyOperationChain { operation_count, compiled } => "carrier-empty-operation-chain" {
            "operation_count" => Unsigned(*operation_count as u128), "compiled" => Boolean(*compiled)
        },
        ChainBoundary { index, preceding_operation, following_operation, target, source } => "carrier-operation-chain-boundary" {
            "index" => Unsigned(*index as u128), "preceding_operation" => Record(preceding_operation),
            "following_operation" => Record(following_operation), "target" => Record(target), "source" => Record(source)
        },
        PreparationPolicy { admitted, expected } => "carrier-preparation-policy" {
            "admitted" => Record(admitted), "expected" => Record(expected)
        },
        UnitFactor { unit, actual, expected, reason } => "carrier-unit-factor" {
            "unit" => Iri(unit), "actual" => Record(actual), "reason" => Record(reason),
            "expected" => Value(match expected {
                Some(value) => purrdf_hash::error::ErrorValue::Record(value),
                None => purrdf_hash::error::ErrorValue::Absent,
            })
        },
        EmptyMetricSources => "carrier-empty-metric-sources" {},
        AmbiguousMetricSurface { first, second } => "carrier-ambiguous-metric-surface" {
            "first" => Iri(first), "second" => Iri(second)
        },
        ImageEndpoints { reason, expected_source, actual_source, expected_target, actual_target } => "carrier-image-endpoints" {
            "reason" => Record(reason),
            "expected_source" => Record(expected_source), "actual_source" => Record(actual_source),
            "expected_target" => Record(expected_target), "actual_target" => Record(actual_target)
        },
        ConflictingOperation { operation, first_source, first_target, first_chain, second_source, second_target, second_chain } => "carrier-conflicting-operation" {
            "operation" => Iri(operation), "first_source" => Iri(first_source), "first_target" => Iri(first_target),
            "first_chain" => Record(first_chain), "second_source" => Iri(second_source), "second_target" => Iri(second_target),
            "second_chain" => Record(second_chain)
        },
        UnknownOperation { operation } => "carrier-unknown-operation" { "operation" => Iri(operation) },
        ReferenceRegistration { reference, actual, expected, reason } => "carrier-reference-registration" {
            "reference" => Iri(reference), "actual" => Record(actual), "expected" => Record(expected), "reason" => Record(reason)
        },
        ReferenceMismatch { reason, actual, expected } => "carrier-reference-mismatch" {
            "reason" => Record(reason), "actual" => Record(actual), "expected" => Record(expected)
        },
        ReferenceIdentity { reason, actual, expected } => "carrier-reference-identity" {
            "reason" => Record(reason), "actual" => Record(actual), "expected" => Record(expected)
        },
        CachedArc { actual_reference, expected_reference, expected, start, end, inverse } => "carrier-cached-arc" {
            "actual_reference" => Record(actual_reference), "expected_reference" => Record(expected_reference),
            "expected" => Record(expected), "start" => Record(start), "end" => Record(end), "inverse" => Record(inverse)
        },
        NonNativeReference { actual } => "carrier-nonnative-reference" { "actual" => Record(actual) },
        InvalidEndpoint { carrier, expected, actual } => "carrier-invalid-endpoint" {
            "carrier" => Iri(carrier), "expected" => Record(expected), "actual" => Record(actual)
        },
        MultipleOperations { source, target, first, second } => "carrier-multiple-operations" {
            "source" => Iri(source), "target" => Iri(target), "first" => Record(first), "second" => Record(second)
        },
        ConflictingEndpoint { carrier, first, second } => "carrier-conflicting-endpoint" {
            "carrier" => Iri(carrier), "first" => Record(first), "second" => Record(second)
        },
    }
}

impl BindingConfigError {
    /// Publish native inline fields and independently retained operand owners.
    pub(crate) fn into_geo(
        self,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        progress.retained_error(|memory| {
            memory
                .boxed(self)
                .map(super::GeoError::BindingConfiguration)
        })
    }

    pub(crate) fn geographic_index_admission(
        target: &crate::Crs,
        references: [&crate::GeographicReference; 2],
        policies: [crate::ExecutionPolicy; 2],
        reason: GeoIndexAdmissionReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let [declared, actual] = references;
        let [expected_policy, actual_policy] = policies;
        capture_references(declared, actual, progress, |declared, actual, progress| {
            Self::GeographicIndexAdmission {
                reason,
                target: target.clone(),
                declared,
                actual,
                expected_policy,
                actual_policy,
            }
            .into_geo(progress)
        })
    }

    pub(crate) fn applicability(
        west: &crate::Rat,
        east: &crate::Rat,
        south: &crate::Rat,
        north: &crate::Rat,
        axis: crate::measure::Ordinate,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let captured = (|| {
            Ok(Self::Applicability {
                axis,
                west: super::GeoError::diagnostic_number(west, progress)?,
                east: super::GeoError::diagnostic_number(east, progress)?,
                south: super::GeoError::diagnostic_number(south, progress)?,
                north: super::GeoError::diagnostic_number(north, progress)?,
            })
        })();
        match captured {
            Ok(cause) => cause.into_geo(progress),
            Err(error) => error,
        }
    }

    pub(crate) fn image_target(
        target: OperationReference,
        reference: Option<&crate::GeographicReference>,
        reason: ImageTargetReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let reference = match reference
            .map(|reference| {
                crate::context::retained::produce_persistent_numeric_from(reference, progress)
                    .map(crate::context::PersistentNumeric::into_value)
            })
            .transpose()
        {
            Ok(reference) => reference,
            Err(error) => return error,
        };
        progress.retained_error(|memory| {
            memory.try_scope(|memory| {
                let metric = match reference {
                    Some(reference) => {
                        crate::operation::ImageMetric::Geographic(memory.boxed(reference)?)
                    }
                    None => crate::operation::ImageMetric::CartesianMetres,
                };
                memory
                    .boxed(Self::ImageTarget {
                        reason,
                        target,
                        metric,
                    })
                    .map(super::GeoError::BindingConfiguration)
            })
        })
    }
    /// Retain the policies compared at the source-admission boundary.
    #[must_use]
    pub fn preparation_policy(
        admitted: crate::ExecutionPolicy,
        expected: crate::ExecutionPolicy,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> super::GeoError {
        let mut progress = crate::context::WorkProgress::new(observer);
        progress.retained_error(|memory| {
            memory
                .boxed(Self::PreparationPolicy { admitted, expected })
                .map(super::GeoError::BindingConfiguration)
        })
    }

    pub(crate) fn reference_registration(
        reference: &crate::Crs,
        actual: &crate::GeographicReference,
        expected: &crate::GeographicReference,
        reason: ReferenceRegistrationReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        capture_references(actual, expected, progress, |actual, expected, progress| {
            progress.retained_error(|memory| {
                memory.try_scope(|memory| {
                    let cause = Self::ReferenceRegistration {
                        reference: memory.string(reference.as_str())?,
                        actual,
                        expected,
                        reason,
                    };
                    memory
                        .boxed(cause)
                        .map(super::GeoError::BindingConfiguration)
                })
            })
        })
    }

    pub(crate) fn reference_mismatch(
        actual: &crate::GeographicReference,
        expected: &crate::GeographicReference,
        reason: ReferenceMismatchReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        capture_references(actual, expected, progress, |actual, expected, progress| {
            Self::ReferenceMismatch {
                reason,
                actual,
                expected,
            }
            .into_geo(progress)
        })
    }

    pub(crate) fn identity_mismatch(
        actual: crate::GeoBindingId,
        expected: crate::GeoBindingId,
        reason: ReferenceMismatchReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        Self::ReferenceIdentity {
            reason,
            actual,
            expected,
        }
        .into_geo(progress)
    }

    pub(crate) fn cached_arc(
        actual_reference: &crate::GeographicReference,
        expected_reference: &crate::GeographicReference,
        expected: crate::GeoBindingId,
        bindings: [crate::GeoBindingId; 3],
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        capture_references(
            actual_reference,
            expected_reference,
            progress,
            |actual_reference, expected_reference, progress| {
                let [start, end, inverse] = bindings;
                Self::CachedArc {
                    actual_reference,
                    expected_reference,
                    expected,
                    start,
                    end,
                    inverse,
                }
                .into_geo(progress)
            },
        )
    }

    pub(crate) fn nonnative_reference(
        actual: &crate::GeographicReference,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let actual =
            match crate::context::retained::produce_persistent_numeric_from(actual, progress) {
                Ok(value) => value.into_value(),
                Err(error) => return error,
            };
        Self::NonNativeReference { actual }.into_geo(progress)
    }

    pub(crate) fn unit_factor(
        unit: &crate::Crs,
        actual: &crate::Rat,
        expected: Option<&crate::Rat>,
        reason: UnitFactorReason,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError {
        let actual = match super::GeoError::diagnostic_number(actual, progress) {
            Ok(value) => value,
            Err(error) => return error,
        };
        let expected = match expected
            .map(|value| super::GeoError::diagnostic_number(value, progress))
            .transpose()
        {
            Ok(value) => value,
            Err(error) => return error,
        };
        progress.retained_error(|memory| {
            memory.try_scope(|memory| {
                let cause = Self::UnitFactor {
                    unit: memory.string(unit.as_str())?,
                    actual,
                    expected,
                    reason,
                };
                memory
                    .boxed(cause)
                    .map(super::GeoError::BindingConfiguration)
            })
        })
    }

    pub(crate) fn native_owned_bytes(&self) -> Result<usize, StorageError> {
        // Exact factors and reference limbs keep the independent native
        // publications that admitted them, as do shared CRS identifiers.
        // Count this box, text and any boxed
        // metric declaration born in this error's own account, once each.
        let payload = match self {
            Self::ImageTarget {
                reason: _,
                target: _,
                metric,
            } => match metric {
                crate::operation::ImageMetric::CartesianMetres => 0,
                crate::operation::ImageMetric::Geographic(_) => {
                    size_of::<crate::GeographicReference>()
                }
            },
            Self::PreparationPolicy { .. }
            | Self::ReferenceMismatch { .. }
            | Self::ReferenceIdentity { .. }
            | Self::CachedArc { .. }
            | Self::NonNativeReference { .. }
            | Self::GeographicIndexAdmission { .. } => 0,
            Self::UnitFactor {
                unit,
                actual: _,
                expected: _,
                reason: _,
            } => unit.capacity(),
            Self::EmptyMetricSources
            | Self::ImageEndpoints { .. }
            | Self::EmptyOperationChain { .. }
            | Self::ChainBoundary { .. }
            | Self::Applicability { .. }
            | Self::OperationUnits { .. }
            | Self::InverseDomain { .. } => 0,
            Self::ConflictingOperation {
                operation,
                first_source,
                first_target,
                first_chain: _,
                second_source,
                second_target,
                second_chain: _,
            } => [
                operation,
                first_source,
                first_target,
                second_source,
                second_target,
            ]
            .into_iter()
            .try_fold(0_usize, |bytes, text| {
                bytes
                    .checked_add(text.capacity())
                    .ok_or(StorageError::SizeOverflow)
            })?,
            Self::AmbiguousMetricSurface { first, second } => first
                .capacity()
                .checked_add(second.capacity())
                .ok_or(StorageError::SizeOverflow)?,
            Self::UnknownOperation { operation } => operation.capacity(),
            Self::ReferenceRegistration {
                reference,
                actual: _,
                expected: _,
                reason: _,
            } => reference.capacity(),
            Self::InvalidEndpoint {
                carrier,
                expected: _,
                actual: _,
            }
            | Self::ConflictingEndpoint {
                carrier,
                first: _,
                second: _,
            } => carrier.capacity(),
            Self::MultipleOperations {
                source,
                target,
                first: _,
                second: _,
            } => source
                .capacity()
                .checked_add(target.capacity())
                .ok_or(StorageError::SizeOverflow)?,
        };
        core::alloc::Layout::new::<Self>()
            .size()
            .checked_add(payload)
            .ok_or(StorageError::SizeOverflow)
    }
}

impl core::fmt::Display for BindingConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::GeographicIndexAdmission { .. } => f.write_str("geographic index requires its actual target reference and original profile policy"),
            Self::InverseDomain { .. } => f.write_str("model requires an explicitly certified inverse domain"),
            Self::ImageTarget { reason, .. } => f.write_str(match reason {
                ImageTargetReason::MetricUnits => "image metric contradicts declared target units",
                ImageTargetReason::MetricReference => "image metric disagrees with actual target realization or axes",
                ImageTargetReason::CurveReference => "image curve requires its actual geographic target",
            }),
            Self::Applicability { .. } => f.write_str("operation applicability must have positive extents"),
            Self::OperationUnits { .. } => f.write_str("operation units disagree with model input/output units"),
            Self::EmptyOperationChain { compiled, .. } => f.write_str(if *compiled { "empty compiled operation chain" } else { "coordinate operation chain is empty" }),
            Self::ChainBoundary { .. } => f.write_str("coordinate operation chain has unmatched reference boundaries"),
            Self::PreparationPolicy { .. } => f.write_str("source admission belongs to a different geographic execution policy"),
            Self::UnitFactor { reason, .. } => f.write_str(match reason {
                UnitFactorReason::NonPositive => "linear output units require a positive exact metre factor",
                UnitFactorReason::OfficialMetre => "the official metre always has factor one",
                UnitFactorReason::ConflictingRegistration => "conflicting linear output-unit registration",
            }),
            Self::EmptyMetricSources => f.write_str("metric reference requires a source carrier"),
            Self::AmbiguousMetricSurface { .. } => f.write_str("multiple reachable physical metric surfaces; name the intended operation image explicitly"),
            Self::ImageEndpoints { reason, .. } => f.write_str(match reason {
                ImageEndpointReason::Chain => "chain endpoints disagree with registered actual carriers",
                ImageEndpointReason::GeographicPreparation => "transformed preparation requires the registered actual geographic target",
            }),
            Self::ConflictingOperation { .. } => f.write_str("conflicting operation name registration"),
            Self::UnknownOperation { operation } => write!(f, "unregistered coordinate operation <{operation}>"),
            Self::ReferenceRegistration { reference, reason, .. } => match reason {
                ReferenceRegistrationReason::Crs84 => f.write_str("CRS84 always denotes WGS84 longitude/latitude"),
                ReferenceRegistrationReason::Epsg4326 => f.write_str("EPSG:4326 registration requires WGS84 with explicit latitude/longitude axes"),
                ReferenceRegistrationReason::ConflictingRegistration => write!(f, "conflicting reference registration for <{reference}>"),
                ReferenceRegistrationReason::ImageEdgeContext => f.write_str("image edge/context target reference mismatch"),
                ReferenceRegistrationReason::TransformedContext => f.write_str("transformed preparation/context target reference mismatch"),
            },
            Self::ReferenceMismatch { reason, .. } | Self::ReferenceIdentity { reason, .. } => core::fmt::Display::fmt(reason, f),
            Self::CachedArc { .. } => f.write_str("cached arc and numerical reference differ"),
            Self::NonNativeReference { .. } => f.write_str("not a native geographic reference"),
            Self::InvalidEndpoint { carrier, .. } => write!(f, "operation endpoint disagrees with declared carrier <{carrier}>"),
            Self::MultipleOperations { .. } => f.write_str("multiple distinct coordinate chains; name the intended operation explicitly"),
            Self::ConflictingEndpoint { .. } => f.write_str("conflicting operation carrier realizations, units or axes"),
        }
    }
}
impl std::error::Error for BindingConfigError {}

/// Keep both independently owned reference publications at the one capture home.
fn capture_references(
    actual: &crate::GeographicReference,
    expected: &crate::GeographicReference,
    progress: &mut crate::context::WorkProgress<'_>,
    make: impl FnOnce(
        crate::GeographicReference,
        crate::GeographicReference,
        &mut crate::context::WorkProgress<'_>,
    ) -> super::GeoError,
) -> super::GeoError {
    let actual = match crate::context::retained::produce_persistent_numeric_from(actual, progress) {
        Ok(value) => value.into_value(),
        Err(error) => return error,
    };
    let expected =
        match crate::context::retained::produce_persistent_numeric_from(expected, progress) {
            Ok(value) => value.into_value(),
            Err(error) => return error,
        };
    make(actual, expected, progress)
}
