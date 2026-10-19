// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared geometry failure channel.
//!
//! Invalid coordinates, literals and operation arguments retain typed source
//! details. The evaluator maps established argument failures to SPARQL expression
//! errors. Missing registrations and inconsistent reference/operation bindings
//! are configuration failures. Numerical uncertainty, resource exhaustion,
//! cancellation, convergence and environment failures are operational: they
//! propagate through FILTER and BIND instead of being mistaken for a negative
//! relation or an unbound value. Offline transformations use explicit compiled
//! models and never infer a datum operation from matching ellipsoid parameters.

mod argument_input;
mod cause;
pub use argument_input::{ArgumentInputError, ArgumentInputReason};
pub use cause::{GeoFailureCause, RetainedGeoCause};
mod unsupported;
pub use unsupported::UnsupportedRefusal;
mod index_config;
pub use index_config::{IndexBindingError, IndexConfigError, IndexSerializationError};
mod matrix;
pub use matrix::MatrixInputError;
mod pattern;
pub use pattern::PatternInputError;
mod binding;
pub use binding::GeoIndexAdmissionReason;
pub use binding::ImageTargetReason;
pub use binding::ReferenceMismatchReason;
pub use binding::{
    BindingConfigError, ImageEndpointReason, ReferenceRegistrationReason, UnitFactorReason,
};
mod ellipsoid;
pub use ellipsoid::{EllipsoidInputError, EllipsoidRequirement};
mod vocabulary;
pub use vocabulary::{VocabularyError, VocabularyNamespace};
mod geometry_domain;
pub use geometry_domain::GeodesicEdgeConsumer;
pub use geometry_domain::PreparedEdgeRequirement;
pub use geometry_domain::WrittenTangentReason;
pub use geometry_domain::{
    DistanceSupportStage, EdgeQuotientOperation, GeometryDomainError, MembershipInput,
};
pub use geometry_domain::{SourceContactCoordinates, SourceContactReason};
mod metric_unit;
pub use metric_unit::{MetricUnitError, MetricUnitReason};
mod metric_input;
pub use metric_input::{DistanceEnclosureReason, MetricInputError};
pub use metric_input::{TravelOperation, UnitParameterOperation, UnitParameterRequirement};
mod source_preparation;
pub use source_preparation::{
    PreparationBaselineReason, PreparationReceiptConsumer, SourcePreparationError,
};
mod context_config;
pub use context_config::{ContextConfigError, OutputReceiptReason, PreparationPhase};
mod transverse;
pub use transverse::{TransverseInputError, TransverseInputReason};
mod affine;
pub use affine::{AffineInputError, HelmertInputReason, OperationValidationContext};
mod projection;
pub use projection::{GcjInputReason, MercatorRequirement, ProjectionInputError};
mod polynomial;
pub use polynomial::PolynomialInputError;
mod model;
pub use model::{GridConfigurationReason, GridDomainSide, ModelInputError};
mod carrier_input;
pub use carrier_input::{CarrierInputError, CarrierInputReason};
mod crs;
pub use crs::CrsError;
mod wkt;
pub use wkt::{WktError, WktOrdinateContext, WktReason};
mod json_syntax;
pub use json_syntax::JsonSyntaxError;
mod geojson_shape;
pub use geojson_shape::GeoJsonShapeError;
mod geojson_write;
pub use geojson_write::GeoJsonWriteError;

/// Why a geometry, metric, coordinate operation or spatial index refused.
///
/// [`Self::is_expression_error`] defines the argument failures admitted by the
/// SPARQL adapter; every other variant remains fatal at that boundary.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GeoError {
    /// The original typed failure and its original native allocation owner.
    /// No rendered copy or detached message replaces that cause.
    Retained(RetainedGeoError),
    /// The input source refused a term or iterator read. No partial index is published.
    SourceRead(GeoFailureCause),
    /// The call supplies the wrong number of arguments for the function named.
    Arity(GeoFailureCause),
    /// Original numeric lexical failure, with its original XSD payload.
    NumericLiteral(RetainedGeoCause),
    /// Original scalar argument term, native requirement and parser failure.
    ArgumentInput(crate::context::Retained<ArgumentInputError>),
    /// The caller's wiring is unusable as written (an empty IRI, a vocabulary
    /// missing a term a registered function needs).
    Config(String),
    /// The caller selected no relation families, so no registration can be made.
    NoRelationFamilies,
    /// Original projection-construction refusal and input operands.
    IndexConfiguration(crate::context::Retained<IndexConfigError>),
    /// Original nonliteral serialization object and configured property.
    IndexSerialization(Box<IndexSerializationError>),
    /// Original index/view binding comparison and lookup invariant.
    IndexBinding(crate::context::Retained<IndexBindingError>),
    /// Original prepared-source receipt or context baseline mismatch.
    SourcePreparation(crate::context::Retained<SourcePreparationError>),
    /// Original worker scratch, backend or owned-storage configuration refusal.
    ContextConfiguration(crate::context::Retained<ContextConfigError>),
    /// Original transverse-projection declaration and source-point refusal.
    TransverseInput(crate::context::Retained<TransverseInputError>),
    /// Original declaration/source failure at the public differential entry.
    DifferentialInput(GeoFailureCause),
    /// Original affine declaration, observation metadata or adjusted-scale refusal.
    AffineInput(crate::context::Retained<AffineInputError>),
    /// Original GCJ applicability or explicit Mercator input refusal.
    ProjectionInput(crate::context::Retained<ProjectionInputError>),
    /// Original polynomial normalization, degree or duplicate-term refusal.
    PolynomialInput(Box<PolynomialInputError>),
    /// Original regular-grid or inverse source-rectangle refusal.
    ModelInput(crate::context::Retained<ModelInputError>),
    /// Original typed validation cause with its existing math-entry context.
    OperationValidation {
        /// Original source/metadata validation context.
        context: OperationValidationContext,
        /// Original native validation refusal and its allocation owner.
        cause: RetainedGeoCause,
    },
    /// Original diagnostic intersection-matrix grammar refusal.
    MatrixConfiguration(crate::context::Retained<MatrixInputError>),
    /// Original carrier-resolution conflict and declarations.
    BindingConfiguration(Box<BindingConfigError>),
    /// Original vocabulary declaration or row-local unit mismatch.
    Vocabulary(crate::context::Retained<VocabularyError>),
    /// Original geometry-domain operands and reference pairs.
    GeometryDomain(crate::context::Retained<GeometryDomainError>),
    /// Original exact metric-unit operands and conversion declaration.
    MetricUnit(crate::context::Retained<MetricUnitError>),
    /// Original distance-certificate operands and failed requirement.
    MetricInput(crate::context::Retained<MetricInputError>),
    /// Original RDF input refused by carrier admission or source ownership.
    CarrierInput(Box<CarrierInputError>),
    /// Original invalid coordinate-reference configuration.
    CrsConfiguration(Box<CrsError>),
    /// The same original refusal in an explicit geometry-literal prefix.
    CrsLiteral {
        /// Byte offset of the original opening angle bracket in the literal.
        prefix_start: usize,
        /// Original identifier and native validator cause.
        cause: Box<CrsError>,
    },
    /// A geometry literal is not well-formed for its datatype.
    Literal(String),
    /// Original model-shape refusal, with its original carrier context.
    GeometryShape(crate::GeometryShapeError),
    /// Original WKT grammar refusal and coordinate context.
    WktSyntax(Box<WktError>),
    /// Original JSON grammar/escape failure beneath a geometry carrier.
    JsonSyntax(Box<JsonSyntaxError>),
    /// Original GeoJSON member, type or coordinate-shape refusal.
    GeoJsonShape(Box<GeoJsonShapeError>),
    /// Original geometry operands refused by GeoJSON output.
    GeoJsonWrite(Box<GeoJsonWriteError>),
    /// Original runtime DE-9IM pattern grammar refusal.
    PatternSyntax(PatternInputError),
    /// A spec-defined operation this crate does not implement. Always loud,
    /// never a default answer.
    Unsupported(UnsupportedRefusal),
    /// Well-formed arguments on which the operation is undefined (mixed
    /// coordinate reference systems, a measure of an empty geometry).
    Domain(std::borrow::Cow<'static, str>),
    /// An original angular coordinate is outside its closed valid range.
    CoordinateOutOfRange {
        /// `longitude` or `latitude`.
        axis: &'static str,
        /// The exact input that was refused.
        value: crate::context::Retained<crate::Rat>,
    },
    /// An IEEE coordinate is NaN or infinity.
    NonFiniteCoordinate {
        /// The coordinate being decoded.
        axis: &'static str,
        /// The original binary64 bits.
        bits: u64,
    },
    /// A promoted distance threshold is NaN or infinity.
    NonFiniteThreshold {
        /// The original binary64 bits.
        bits: u64,
    },
    /// Numeric threshold promotion received a value outside the numeric tower.
    NonNumericThreshold {
        /// Original parsed XSD datatype.
        datatype: purrdf_xsd::XsdDatatype,
    },
    /// A carrier names a reference the caller has not registered.
    UnregisteredCrs(String),
    /// Comparing reference systems requires an explicit operation chain.
    MissingOperation {
        /// The source reference identifier.
        source: String,
        /// The target reference identifier.
        target: String,
    },
    /// A three-dimensional operation requires an actual height.
    MissingHeight,
    /// Endpoint-only construction does not select a unique geodesic arc.
    AmbiguousGeodesic,
    /// The admitted transformation domain contains multiple inverse roots.
    AmbiguousTransform {
        /// The number of separately certified admissible roots.
        roots: u64,
    },
    /// The declared ellipsoid parameters are invalid.
    InvalidEllipsoid(crate::context::Retained<EllipsoidInputError>),
    /// The execution policy cannot admit a complete operation.
    InvalidExecutionPolicy(crate::ExecutionLimits),
    /// Original invalid mixed-cover limits, before any output allocation.
    InvalidCoverPolicy(crate::cells::MixedCoverLimits),
    /// Original invalid point-index limits, before any input allocation.
    InvalidPointIndexPolicy(crate::cells::PointIndexLimits),
    /// A bounded producer cannot transfer its original live allocation owner.
    InvalidNativeStorage(&'static str),
    /// A native geodesic entry received borrowed progress instead of its owner.
    MissingNativeObserver {
        /// Original native entry operation.
        operation: crate::geodesic::NativeEntryOperation,
    },
    /// A completed inverse solution has no required captured metadata.
    MissingInverseMetadata,
    /// The certified enclosure could not resolve the required rounding.
    PrecisionExhausted {
        /// The admitted maximum precision in bits.
        bits: u32,
    },
    /// The operation exceeded its admitted work.
    WorkExhausted {
        /// The admitted work-item count.
        limit: u64,
    },
    /// The operation exceeded its admitted retained workspace.
    MemoryExhausted {
        /// The admitted byte count.
        limit: u64,
    },
    /// The explicit reusable integer scratch could not admit a destination.
    /// No hidden allocation or last-iterate approximation replaces this refusal.
    NumericalScratch(purrdf_xsd::integer::LimbScratchError),
    /// A reusable Taylor pool is already checked out by another branded scope.
    NumericalTaylorScratch(purrdf_xsd::math::TaylorScratchError),
    /// Original numerical cause without a lossy outer-category substitution.
    NativeMath(purrdf_xsd::math::MathError),
    /// A native destination or original physical admission refused before birth.
    Storage(purrdf_lex::allocation::StorageError),
    /// A complete result exceeds the admitted output count.
    OutputExhausted {
        /// The admitted output count.
        limit: u64,
    },
    /// Cancellation prevents publication of a complete result.
    Cancelled,
    /// The solver exhausted its admitted iterations without a certificate.
    ConvergenceExhausted {
        /// The admitted iteration count.
        iterations: u32,
    },
    /// The calling thread cannot run the declared IEEE numerical law.
    FloatEnvironment(purrdf_xsd::ieee::environment::FloatEnvironmentError),
    /// A checked count or arithmetic intermediate cannot be represented.
    ArithmeticOverflow(&'static str),
    /// A physical maximum-edge target is not strictly positive.
    NonPositiveEdgeLength(crate::context::Retained<crate::Rat>),
    /// No admitted native hierarchy level reaches this physical target.
    UnattainableEdgeLength {
        /// The requested guaranteed upper edge length in metres.
        target: crate::context::Retained<crate::Rat>,
        /// The level-30 guaranteed upper bound in metres.
        minimum: crate::context::Retained<crate::Rat>,
    },
    /// A native cube hierarchy level is outside `0..=30`.
    InvalidResolution(u8),
    /// A key has an invalid face, sentinel or bit layout.
    InvalidCellId(u64),
    /// Cell and grid identities name different profiles.
    GridProfileMismatch,
    /// The stored resolution cannot represent this cell's descendants.
    InvalidStoredLevel {
        /// This cell's level.
        cell: u8,
        /// The requested storage level.
        stored: u8,
    },
    /// An ancestor request names a finer level than the input cell.
    InvalidAncestorLevel {
        /// This cell's level.
        cell: u8,
        /// The requested ancestor level.
        ancestor: u8,
    },
    /// A root cell has no parent.
    RootHasNoParent,
    /// A level-30 cell has no children.
    LeafHasNoChildren,
    /// A caller-owned batch output buffer has the wrong length.
    InvalidOutputLength {
        /// Required element count.
        expected: usize,
        /// Supplied element count.
        actual: usize,
    },
    /// A physical disk radius is negative.
    NegativePhysicalRadius(crate::context::Retained<crate::Rat>),
    /// Mixed cover levels do not form a supported closed interval.
    InvalidCoverLevels {
        /// Lowest emitted level.
        min: u8,
        /// Finest geometric classification level.
        max: u8,
    },
    /// The complete canonical cover exceeds its admitted cell count.
    CoverCellsExhausted {
        /// Final emitted-cell or fixed logical-cell admission.
        limit: u64,
    },
    /// A point index contains a repeated caller point key.
    DuplicatePointKey(u64),
    /// The declared point reference is not this native grid's reference.
    PointIndexReferenceMismatch,
}

impl purrdf_hash::error::StructuredError for GeoError {
    purrdf_hash::error_visit! {
        Retained(cause) => "geo-retained" { "cause" => Cause(cause) },
        SourceRead(cause) => "geo-source-read" { "cause" => Cause(cause) },
        Arity(cause) => "geo-arity" { "cause" => Cause(cause) },
        NumericLiteral(cause) => "geo-numeric-literal" { "cause" => Cause(cause) },
        ArgumentInput(cause) => "geo-argument-input-error" { "cause" => Cause(&**cause) },
        Config(reason) => "geo-config" { "reason" => Text(reason) },
        NoRelationFamilies => "geo-no-relation-families" {},
        IndexConfiguration(cause) => "geo-index-configuration" { "cause" => Cause(&**cause) },
        IndexSerialization(cause) => "geo-index-serialization" { "cause" => Cause(&**cause) },
        IndexBinding(cause) => "geo-index-binding" { "cause" => Cause(&**cause) },
        SourcePreparation(cause) => "geo-source-preparation" { "cause" => Cause(&**cause) },
        ContextConfiguration(cause) => "geo-context-configuration" { "cause" => Cause(&**cause) },
        TransverseInput(cause) => "geo-transverse-input" { "cause" => Cause(&**cause) },
        DifferentialInput(cause) => "geo-differential-input" { "cause" => Cause(cause) },
        AffineInput(cause) => "geo-affine-input" { "cause" => Cause(&**cause) },
        ProjectionInput(cause) => "geo-projection-input" { "cause" => Cause(&**cause) },
        PolynomialInput(cause) => "geo-polynomial-input" { "cause" => Cause(&**cause) },
        ModelInput(cause) => "geo-model-input" { "cause" => Cause(&**cause) },
        OperationValidation { context, cause } => "geo-operation-validation" { "context" => Record(context), "cause" => Cause(cause) },
        MatrixConfiguration(cause) => "geo-matrix-configuration" { "cause" => Cause(&**cause) },
        BindingConfiguration(cause) => "geo-binding-configuration" { "cause" => Cause(&**cause) },
        Vocabulary(cause) => "geo-vocabulary" { "cause" => Cause(&**cause) },
        GeometryDomain(cause) => "geo-geometry-domain" { "cause" => Cause(&**cause) },
        MetricUnit(cause) => "geo-metric-unit" { "cause" => Cause(&**cause) },
        MetricInput(cause) => "geo-metric-input" { "cause" => Cause(&**cause) },
        CarrierInput(cause) => "geo-carrier-input" { "cause" => Cause(&**cause) },
        CrsConfiguration(cause) => "geo-crs-configuration" { "cause" => Cause(&**cause) },
        CrsLiteral { prefix_start, cause } => "geo-crs-literal" {
            "prefix_start" => Unsigned(*prefix_start as u128), "cause" => Cause(&**cause)
        },
        Literal(reason) => "geo-literal" { "reason" => Text(reason) },
        GeometryShape(cause) => "geo-geometry-shape" { "cause" => Cause(cause) },
        WktSyntax(cause) => "geo-wkt-syntax" { "cause" => Cause(&**cause) },
        JsonSyntax(cause) => "geo-json-reader" { "cause" => Cause(&**cause) },
        GeoJsonShape(cause) => "geo-geojson-shape" { "cause" => Cause(&**cause) },
        GeoJsonWrite(cause) => "geo-geojson-write" { "cause" => Cause(&**cause) },
        PatternSyntax(cause) => "geo-pattern-syntax" { "cause" => Cause(cause) },
        Unsupported(cause) => "geo-unsupported" { "cause" => Cause(cause) },
        Domain(reason) => "geo-domain" { "reason" => Text(reason.as_ref()) },
        CoordinateOutOfRange { axis, value } => "geo-coordinate-out-of-range" { "axis" => Code(axis), "value" => Record(value) },
        NonFiniteCoordinate { axis, bits } => "geo-non-finite-coordinate" { "axis" => Code(axis), "bits" => Float64Bits(*bits) },
        NonFiniteThreshold { bits } => "geo-non-finite-threshold" { "bits" => Float64Bits(*bits) },
        NonNumericThreshold { datatype } => "geo-nonnumeric-threshold" { "datatype" => Iri(datatype.iri()) },
        UnregisteredCrs(iri) => "geo-unregistered-crs" { "iri" => Iri(iri) },
        MissingOperation { source, target } => "geo-missing-operation" { "source" => Iri(source), "target" => Iri(target) },
        MissingHeight => "geo-missing-height" {},
        AmbiguousGeodesic => "geo-ambiguous-geodesic" {},
        AmbiguousTransform { roots } => "geo-ambiguous-transform" { "roots" => Unsigned(u128::from(*roots)) },
        InvalidEllipsoid(cause) => "geo-invalid-ellipsoid" { "cause" => Cause(&**cause) },
        InvalidExecutionPolicy(limits) => "geo-invalid-execution-policy" { "limits" => Record(limits) },
        InvalidCoverPolicy(limits) => "geo-invalid-cover-policy" { "limits" => Record(limits) },
        InvalidPointIndexPolicy(limits) => "geo-invalid-point-index-policy" { "limits" => Record(limits) },
        InvalidNativeStorage(reason) => "geo-invalid-native-storage" { "reason" => Text(reason) },
        MissingNativeObserver { operation } => "geo-missing-native-observer" { "operation" => Record(operation) },
        MissingInverseMetadata => "geo-missing-inverse-metadata" {},
        PrecisionExhausted { bits } => "geo-precision-exhausted" { "bits" => Unsigned(u128::from(*bits)) },
        WorkExhausted { limit } => "geo-work-exhausted" { "limit" => Unsigned(u128::from(*limit)) },
        MemoryExhausted { limit } => "geo-memory-exhausted" { "limit" => ByteCount(u128::from(*limit)) },
        NumericalScratch(cause) => "geo-numerical-scratch" { "cause" => Cause(cause) },
        NumericalTaylorScratch(cause) => "geo-numerical-taylor-scratch" { "cause" => Cause(cause) },
        NativeMath(cause) => "geo-native-math" { "cause" => Cause(cause) },
        Storage(cause) => "geo-storage" { "cause" => Cause(cause) },
        OutputExhausted { limit } => "geo-output-exhausted" { "limit" => Unsigned(u128::from(*limit)) },
        Cancelled => "geo-cancelled" {},
        ConvergenceExhausted { iterations } => "geo-convergence-exhausted" { "iterations" => Unsigned(u128::from(*iterations)) },
        FloatEnvironment(cause) => "geo-float-environment" { "cause" => Cause(cause) },
        ArithmeticOverflow(reason) => "geo-arithmetic-overflow" { "reason" => Text(reason) },
        NonPositiveEdgeLength(value) => "geo-non-positive-edge-length" { "value" => Record(value) },
        UnattainableEdgeLength { target, minimum } => "geo-unattainable-edge-length" { "target" => Record(target), "minimum" => Record(minimum) },
        InvalidResolution(level) => "geo-invalid-resolution" { "level" => Unsigned(u128::from(*level)) },
        InvalidCellId(id) => "geo-invalid-cell-id" { "id" => Unsigned(u128::from(*id)) },
        GridProfileMismatch => "geo-grid-profile-mismatch" {},
        InvalidStoredLevel { cell, stored } => "geo-invalid-stored-level" { "cell" => Unsigned(u128::from(*cell)), "stored" => Unsigned(u128::from(*stored)) },
        InvalidAncestorLevel { cell, ancestor } => "geo-invalid-ancestor-level" { "cell" => Unsigned(u128::from(*cell)), "ancestor" => Unsigned(u128::from(*ancestor)) },
        RootHasNoParent => "geo-root-has-no-parent" {},
        LeafHasNoChildren => "geo-leaf-has-no-children" {},
        InvalidOutputLength { expected, actual } => "geo-invalid-output-length" { "expected" => Unsigned(*expected as u128), "actual" => Unsigned(*actual as u128) },
        NegativePhysicalRadius(value) => "geo-negative-physical-radius" { "value" => Record(value) },
        InvalidCoverLevels { min, max } => "geo-invalid-cover-levels" { "min" => Unsigned(u128::from(*min)), "max" => Unsigned(u128::from(*max)) },
        CoverCellsExhausted { limit } => "geo-cover-cells-exhausted" { "limit" => Unsigned(u128::from(*limit)) },
        DuplicatePointKey(key) => "geo-duplicate-point-key" { "key" => Unsigned(u128::from(*key)) },
        PointIndexReferenceMismatch => "geo-point-index-reference-mismatch" {},
    }
}

/// A failure payload retaining the account under which its fields were born.
#[derive(Clone, Debug)]
pub struct RetainedGeoError(pub(crate) purrdf_lex::allocation::SharedOwned<GeoError>);

impl purrdf_hash::error::StructuredError for RetainedGeoError {
    purrdf_hash::error_visit! {
        @tuple self (_owner) => "geo-retained-error" { "cause" => Cause(self.original()) }
    }
}

impl RetainedGeoError {
    /// Borrow the original typed cause without detaching its allocation owner.
    #[must_use]
    pub fn original(&self) -> &GeoError {
        self.0.original()
    }
}

impl GeoError {
    /// Publish an already caller-owned resident diagnostic without copying it.
    /// Bounded producers use their original retained-allocation factory.
    pub(crate) fn resident_failure<T: purrdf_hash::error::NativeError + 'static>(
        cause: T,
        make: fn(crate::context::Retained<T>) -> Self,
    ) -> Self {
        match crate::context::Retained::try_new(cause) {
            Ok(cause) => make(cause),
            Err(error) => error,
        }
    }

    /// Publish an already constructed inline native cause under its original
    /// physical account. Any owned heap fields must already have their owner;
    /// this factory does not admit prior resident copies for a bounded caller.
    pub(crate) fn retained_failure<T: purrdf_hash::error::NativeError + 'static>(
        cause: T,
        make: fn(crate::context::Retained<T>) -> Self,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Self {
        let mut progress = crate::context::WorkProgress::new(observer);
        Self::retained_failure_with_progress(cause, make, &mut progress)
    }

    /// The same native publication within an already active progress owner.
    pub(crate) fn retained_failure_with_progress<T: purrdf_hash::error::NativeError + 'static>(
        cause: T,
        make: fn(crate::context::Retained<T>) -> Self,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Self {
        match crate::context::Retained::produce(progress, |_| Ok(cause), |_, _| Ok(())) {
            Ok(cause) => make(cause),
            Err(error) => error,
        }
    }

    /// Retain original member/extent operands without rendering a message.
    #[must_use]
    pub fn geometry_domain_metered(
        cause: GeometryDomainError,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Self {
        Self::retained_failure(cause, Self::GeometryDomain, observer)
    }

    /// Copy an exact diagnostic operand inside its original physical factory.
    /// The immutable control and detached coefficient buffers are admitted
    /// before birth; clones retain that same original error payload owner.
    pub(crate) fn diagnostic_number(
        value: &crate::Rat,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<crate::context::Retained<crate::Rat>, Self> {
        crate::context::Retained::produce(
            progress,
            |memory| value.detached_with_memory(memory).map_err(Self::Storage),
            |value, original| value.publish_detached(original).map_err(Self::Storage),
        )
    }

    /// Capture scalar declarations, optional source data and an exact computed
    /// operand through their original detached numerical publication home.
    pub(crate) fn diagnostic_inputs<T: crate::context::retained::NativeDetached>(
        declaration: &T,
        point: Option<&crate::operation::OperationPoint>,
        comparison: Option<&crate::Rat>,
        progress: &mut crate::context::WorkProgress<'_>,
        make: impl FnOnce(
            T,
            Option<crate::operation::OperationPoint>,
            Option<crate::context::Retained<crate::Rat>>,
            &mut crate::context::WorkProgress<'_>,
        ) -> Self,
    ) -> Self {
        let captured = (|| {
            let declaration =
                crate::context::retained::produce_persistent_numeric_from(declaration, progress)?
                    .into_value();
            let point = point
                .map(|value| {
                    crate::context::retained::produce_persistent_numeric_from(value, progress)
                        .map(crate::context::PersistentNumeric::into_value)
                })
                .transpose()?;
            let comparison = comparison
                .map(|value| Self::diagnostic_number(value, progress))
                .transpose()?;
            Ok((declaration, point, comparison))
        })();
        match captured {
            Ok((declaration, point, comparison)) => make(declaration, point, comparison, progress),
            Err(error) => error,
        }
    }

    /// Carry a native domain refusal through an existing numerical boundary;
    /// resource failures keep their original operational category and owner.
    pub(crate) fn math_validation_failure(
        error: Self,
        context: OperationValidationContext,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> purrdf_xsd::math::MathError {
        let error = if error.is_expression_error() {
            match RetainedGeoCause::capture(error, progress) {
                Ok(cause) => Self::OperationValidation { context, cause },
                Err(error) => error,
            }
        } else {
            error
        };
        progress.math_failure(error)
    }
}
impl PartialEq for RetainedGeoError {
    fn eq(&self, other: &Self) -> bool {
        self.original() == other.original()
    }
}
impl Eq for RetainedGeoError {}
impl core::hash::Hash for RetainedGeoError {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(self.original(), state);
    }
}

purrdf_lex::constructors! {
    impl GeoError {

        /// A [`GeoError::Config`] with `what` as its detail.
        pub fn config(what) -> Self::Config;

        /// A [`GeoError::Literal`] with `what` as its detail.
        pub fn literal(what) -> Self::Literal;


        /// A [`GeoError::Domain`] with `what` as its detail.
        pub fn domain(what) -> Self::Domain;
    }
}

impl GeoError {
    /// Construct a source failure from genuine caller message data.
    /// Native dataset reads use [`Self::native_source`] instead.
    pub fn source_read(message: impl Into<String>) -> Self {
        Self::SourceRead(GeoFailureCause::Message(message.into()))
    }

    /// Construct an arity refusal from genuine caller message data.
    /// Native function checks retain their original arity through
    /// [`Self::from_native_cause_metered`].
    pub fn arity(message: impl Into<String>) -> Self {
        Self::Arity(GeoFailureCause::Message(message.into()))
    }

    /// An unsupported operation described by genuine caller message data.
    /// Native producers retain their specific [`UnsupportedRefusal`] instead.
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported(UnsupportedRefusal::Message(message.into()))
    }

    /// Preserve an original native cause through the existing physical owner.
    /// The constructor selects the established geographic failure category.
    /// No cause is rendered during capture or inspection.
    #[must_use]
    pub fn from_native_cause_metered(
        make: fn(GeoFailureCause) -> Self,
        cause: impl purrdf_hash::error::NativeError + 'static,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Self {
        Self::from_native_cause_with_progress(
            make,
            cause,
            &mut crate::context::WorkProgress::integer(observer),
        )
    }

    fn from_native_cause_with_progress(
        make: fn(GeoFailureCause) -> Self,
        cause: impl purrdf_hash::error::NativeError + 'static,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Self {
        match RetainedGeoCause::capture(cause, progress) {
            Ok(cause) => make(GeoFailureCause::Native(cause)),
            Err(error) => error,
        }
    }

    /// Retain an original resident source cause without rendering it.
    #[must_use]
    pub fn native_source(cause: impl purrdf_hash::error::NativeError + 'static) -> Self {
        Self::native_source_with_progress(cause, &mut crate::context::WorkProgress::integer(None))
    }

    pub(crate) fn native_source_with_progress(
        cause: impl purrdf_hash::error::NativeError + 'static,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Self {
        Self::from_native_cause_with_progress(Self::SourceRead, cause, progress)
    }

    /// Borrow the authoritative typed cause, preserving its original owner.
    #[must_use]
    pub fn original(&self) -> &Self {
        let mut error = self;
        while let Self::Retained(original) = error {
            error = &original.0;
        }
        error
    }

    pub(crate) fn native_text_bytes(&self) -> Result<usize, purrdf_lex::allocation::StorageError> {
        match self {
            Self::SourceRead(cause) | Self::Arity(cause) | Self::DifferentialInput(cause) => {
                Ok(cause.native_text_bytes())
            }
            Self::Unsupported(cause) => Ok(cause.native_text_bytes()),
            Self::PatternSyntax(cause) => Ok(cause.native_text_bytes()),
            Self::BindingConfiguration(cause) => cause.native_owned_bytes(),
            Self::PolynomialInput(cause) => cause.native_owned_bytes(),
            Self::CarrierInput(cause) => Ok(cause.owned_bytes()),
            Self::IndexSerialization(cause) => Ok(cause.owned_bytes()),
            Self::CrsConfiguration(cause) | Self::CrsLiteral { cause, .. } => cause.owned_bytes(),
            Self::WktSyntax(cause) => cause.owned_bytes(),
            Self::JsonSyntax(_) => Ok(size_of::<JsonSyntaxError>()),
            Self::GeoJsonShape(cause) => cause.owned_bytes(),
            Self::GeoJsonWrite(cause) => cause.owned_bytes(),
            Self::Config(text) | Self::Literal(text) | Self::UnregisteredCrs(text) => {
                Ok(text.capacity())
            }
            Self::Domain(std::borrow::Cow::Owned(text)) => Ok(text.capacity()),
            Self::MissingOperation { source, target } => source
                .capacity()
                .checked_add(target.capacity())
                .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow),
            _ => Ok(0),
        }
    }
    /// Whether this refusal is a SPARQL **expression error** — scoped to the one
    /// solution being evaluated — rather than a condition that must abort the whole
    /// query.
    ///
    /// The evaluator uses this one kernel classification on every call path.
    /// A failure that aborts a query on one path and drops a row on another
    /// would be unreliable. This shared decision makes the `geof:` seam obey
    /// SPARQL 1.1 §17.2: an invalid argument in one solution does not abort
    /// unrelated solutions, while an operational refusal remains fatal.
    ///
    /// * [`Domain`](Self::Domain) and [`Literal`](Self::Literal) are expression
    ///   errors. Both are statements about *these arguments*: a geometry literal
    ///   whose lexical form its datatype does not license is exactly SPARQL's
    ///   ill-typed literal, and §17.2's "Functions invoked with an argument of the
    ///   wrong type will produce a type error" puts it in the per-solution channel —
    ///   the same channel `"abc"^^xsd:integer > 1` lands in. Making them fatal would
    ///   mean one malformed geometry anywhere in a dataset kills every query that
    ///   scans past it, which is a much larger claim than the data supports.
    /// * [`Arity`](Self::Arity), [`Unsupported`](Self::Unsupported) and
    ///   [`Config`](Self::Config) are NOT. Each holds for *every* solution alike, so
    ///   answering "no value" would empty a result set and present that as the
    ///   answer, reopening the silent-wrong-answer channel this crate exists to keep
    ///   shut. A wrong argument count is a defect in the query text that no row can
    ///   satisfy. An unimplemented function that answered "no
    ///   value" would be dropped by a `FILTER` and left unbound by a `BIND` — in
    ///   both cases indistinguishable from an honest negative result, with nothing
    ///   downstream able to tell the difference. A `Config` refusal names a
    ///   declaration the host never made — an undeclared linear unit, an absent
    ///   Simple Features namespace — and the host is the only party who can make it;
    ///   PurRDF fabricates no vocabulary defaults to fall back on. A row may be what
    ///   *reveals* the gap, but leaving that row unbound would report "this geometry
    ///   has no area" and hide it.
    #[must_use]
    pub fn is_expression_error(&self) -> bool {
        if self.is_domain_error() {
            return true;
        }
        if let Self::Vocabulary(cause) = self.original() {
            return cause.is_expression_error();
        }
        if let Self::CarrierInput(cause) = self.original() {
            return !cause.is_configuration();
        }
        matches!(
            self.original(),
            Self::CrsLiteral { .. }
                | Self::WktSyntax(_)
                | Self::JsonSyntax(_)
                | Self::GeometryShape(_)
                | Self::GeoJsonShape(_)
                | Self::IndexSerialization(_)
                | Self::Literal(_)
                | Self::NumericLiteral(_)
                | Self::ArgumentInput(_)
                | Self::NativeMath(purrdf_xsd::math::MathError::AmbiguousRoots { .. })
                | Self::PatternSyntax(_)
                | Self::CoordinateOutOfRange { .. }
                | Self::NonFiniteCoordinate { .. }
                | Self::NonFiniteThreshold { .. }
                | Self::MissingHeight
                | Self::AmbiguousGeodesic
                | Self::AmbiguousTransform { .. }
                | Self::NegativePhysicalRadius(_)
        )
    }

    /// Whether the original operands fall outside an operation's value domain.
    /// Carrier syntax, configuration and operational refusals remain distinct.
    #[must_use]
    pub fn is_domain_error(&self) -> bool {
        match self.original() {
            Self::Domain(_)
            | Self::GeometryDomain(_)
            | Self::GeoJsonWrite(_)
            | Self::MetricInput(_)
            | Self::MissingInverseMetadata
            | Self::DifferentialInput(_)
            | Self::OperationValidation { .. }
            | Self::NonNumericThreshold { .. }
            | Self::NativeMath(purrdf_xsd::math::MathError::Domain(_)) => true,
            Self::Vocabulary(cause) => cause.is_expression_error(),
            Self::TransverseInput(cause) => cause.is_domain_error(),
            Self::AffineInput(cause) => cause.is_domain_error(),
            Self::ProjectionInput(cause) => cause.is_domain_error(),
            Self::ModelInput(cause) => cause.is_domain_error(),
            _ => false,
        }
    }

    /// The detail message, without this error's own prefix.
    /// Native causes return a separate owned human presentation. Inspection
    /// never calls this accessor; bounded output uses [`Self::message_with_memory`].
    #[must_use]
    pub fn detail(&self) -> std::borrow::Cow<'_, str> {
        let text: &str = match self {
            Self::Retained(original) => return original.original().detail(),
            Self::SourceRead(cause) | Self::Arity(cause) => return cause.detail(),
            Self::NumericLiteral(cause) => return std::borrow::Cow::Owned(cause.to_string()),
            Self::NativeMath(purrdf_xsd::math::MathError::Domain(reason)) => reason,
            Self::NativeMath(cause) => return std::borrow::Cow::Owned(cause.to_string()),
            Self::Unsupported(cause) => {
                return match cause {
                    UnsupportedRefusal::Message(message) => std::borrow::Cow::Borrowed(message),
                    other => std::borrow::Cow::Owned(other.to_string()),
                };
            }
            Self::IndexConfiguration(cause) => {
                return std::borrow::Cow::Owned((**cause).to_string());
            }
            Self::MatrixConfiguration(cause) => {
                return std::borrow::Cow::Owned((**cause).to_string());
            }
            Self::PatternSyntax(cause) => return std::borrow::Cow::Owned(cause.to_string()),
            Self::BindingConfiguration(cause) => {
                return std::borrow::Cow::Owned((**cause).to_string());
            }
            Self::Vocabulary(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::GeometryDomain(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::MetricUnit(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::MetricInput(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::CarrierInput(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::CrsConfiguration(cause) | Self::CrsLiteral { cause, .. } => {
                return std::borrow::Cow::Owned((**cause).to_string());
            }
            Self::WktSyntax(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::JsonSyntax(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::GeometryShape(cause) => return std::borrow::Cow::Owned(cause.to_string()),
            Self::GeoJsonShape(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::GeoJsonWrite(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::IndexSerialization(cause) => {
                return std::borrow::Cow::Owned((**cause).to_string());
            }
            Self::IndexBinding(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::SourcePreparation(cause) => {
                return std::borrow::Cow::Owned((**cause).to_string());
            }
            Self::ContextConfiguration(cause) => {
                return std::borrow::Cow::Owned((**cause).to_string());
            }
            Self::TransverseInput(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::AffineInput(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::ProjectionInput(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::PolynomialInput(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::ModelInput(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::OperationValidation { context, .. } => context.detail(),
            Self::Config(msg) | Self::Literal(msg) => msg,
            Self::ArgumentInput(cause) => return std::borrow::Cow::Owned((**cause).to_string()),
            Self::NoRelationFamilies => {
                "no relation families supplied to register; registering nothing and returning success would surface much later as a query whose geo: relation was parsed as an ordinary triple pattern and matched nothing"
            }
            Self::Domain(msg) => msg,
            Self::UnregisteredCrs(msg) => msg,
            Self::InvalidExecutionPolicy(_) => "admission limits must be positive",
            Self::InvalidCoverPolicy(_) => "cover admission limits must be positive",
            Self::InvalidPointIndexPolicy(_) => "point-index limits must be positive",
            Self::InvalidEllipsoid(cause) => cause.requirement.message(),
            Self::InvalidNativeStorage(msg) | Self::ArithmeticOverflow(msg) => msg,
            Self::CoordinateOutOfRange { .. } => "original angular coordinate outside its range",
            Self::NonFiniteCoordinate { .. } => "non-finite angular coordinate",
            Self::NonFiniteThreshold { .. } => "non-finite distance threshold",
            Self::NonNumericThreshold { .. } => "a distance threshold must be numeric",
            Self::MissingOperation { .. } => "an explicit coordinate operation is required",
            Self::MissingHeight => "an actual height is required",
            Self::AmbiguousGeodesic => "endpoints do not select a unique geodesic arc",
            Self::AmbiguousTransform { .. } => "multiple admissible inverse roots",
            Self::PrecisionExhausted { .. } => "required rounding remains uncertified",
            Self::WorkExhausted { .. } => "admitted work exhausted",
            Self::MemoryExhausted { .. } => "admitted workspace exhausted",
            Self::NumericalScratch(_) => "admitted integer scratch refused a destination",
            Self::NumericalTaylorScratch(_) => "admitted Taylor scratch is already in use",
            Self::MissingNativeObserver { .. } => "native entry requires its owned observer",
            Self::MissingInverseMetadata => "inverse metadata capture missing",
            Self::DifferentialInput(_) => "invalid declared projection or source point",
            Self::Storage(_) => "native geometry storage refused",
            Self::OutputExhausted { .. } => "complete output exceeds its admission",
            Self::Cancelled => "operation cancelled before completion",
            Self::ConvergenceExhausted { .. } => "solver did not certify convergence",
            Self::FloatEnvironment(_) => "invalid calling-thread floating-point environment",
            Self::NonPositiveEdgeLength(_) => "maximum edge length must be positive",
            Self::UnattainableEdgeLength { .. } => "maximum edge target is finer than level 30",
            Self::InvalidResolution(_) => "hierarchy level outside 0..=30",
            Self::InvalidCellId(_) => "invalid native cell key",
            Self::GridProfileMismatch => "cell and grid profiles differ",
            Self::InvalidStoredLevel { .. } => "stored level is shallower than the cover cell",
            Self::InvalidAncestorLevel { .. } => "ancestor level is finer than the input cell",
            Self::RootHasNoParent => "a root cell has no parent",
            Self::LeafHasNoChildren => "a leaf cell has no children",
            Self::InvalidOutputLength { .. } => "batch output buffer has the wrong length",
            Self::NegativePhysicalRadius(_) => "physical radius must be nonnegative",
            Self::InvalidCoverLevels { .. } => "invalid mixed cover level interval",
            Self::CoverCellsExhausted { .. } => "complete canonical cover exceeds cell admission",
            Self::DuplicatePointKey(_) => "point index caller keys must be unique",
            Self::PointIndexReferenceMismatch => "point index reference differs from native grid",
        };
        std::borrow::Cow::Borrowed(text)
    }
}

impl GeoError {
    fn fmt_with<'a, D: core::fmt::Display>(
        &'a self,
        f: &mut core::fmt::Formatter<'_>,
        mut number: impl FnMut(std::borrow::Cow<'a, crate::Int>) -> D,
    ) -> core::fmt::Result {
        match self {
            Self::Retained(original) => original.original().fmt_with(f, number),
            Self::SourceRead(msg) => write!(f, "geometry source read error: {msg}"),
            Self::Arity(msg) => write!(f, "wrong argument count: {msg}"),
            Self::NumericLiteral(cause) => write!(f, "malformed geometry literal: {cause}"),
            Self::ArgumentInput(cause) => write!(f, "malformed geometry literal: {}", **cause),
            Self::Config(msg) => write!(f, "invalid geo configuration: {msg}"),
            Self::NoRelationFamilies => write!(f, "invalid geo configuration: {}", self.detail()),
            Self::CrsConfiguration(cause) => write!(f, "invalid geo configuration: {}", **cause),
            Self::CrsLiteral { cause, .. } => write!(f, "malformed geometry literal: {}", **cause),
            Self::IndexConfiguration(cause) => write!(f, "invalid geo configuration: {}", **cause),
            Self::MatrixConfiguration(cause) => write!(f, "invalid geo configuration: {}", **cause),
            Self::BindingConfiguration(cause) => {
                write!(f, "invalid geo configuration: {}", **cause)
            }
            Self::Vocabulary(cause) => {
                let prefix = if cause.is_expression_error() {
                    "geometry domain error"
                } else {
                    "invalid geo configuration"
                };
                write!(f, "{prefix}: {}", **cause)
            }
            Self::Literal(msg) => write!(f, "malformed geometry literal: {msg}"),
            Self::GeometryShape(cause) => write!(f, "malformed geometry literal: {cause}"),
            Self::GeoJsonShape(cause) => write!(f, "malformed geometry literal: {}", **cause),
            Self::GeoJsonWrite(cause) => write!(f, "geometry domain error: {}", **cause),
            Self::IndexSerialization(cause) => write!(f, "malformed geometry literal: {}", **cause),
            Self::IndexBinding(cause) => write!(f, "invalid geo configuration: {}", **cause),
            Self::SourcePreparation(cause) => write!(f, "invalid geo configuration: {}", **cause),
            Self::ContextConfiguration(cause) => {
                write!(f, "invalid geo configuration: {}", **cause)
            }
            Self::TransverseInput(cause) => write!(
                f,
                "{}: {}",
                if cause.is_domain_error() {
                    "geometry domain error"
                } else {
                    "invalid geo configuration"
                },
                **cause
            ),
            Self::AffineInput(cause) => write!(
                f,
                "{}: {}",
                if cause.is_domain_error() {
                    "geometry domain error"
                } else {
                    "invalid geo configuration"
                },
                **cause
            ),
            Self::ProjectionInput(cause) => write!(
                f,
                "{}: {}",
                if cause.is_domain_error() {
                    "geometry domain error"
                } else {
                    "invalid geo configuration"
                },
                **cause
            ),
            Self::PolynomialInput(cause) => write!(f, "invalid geo configuration: {}", **cause),
            Self::ModelInput(cause) => write!(
                f,
                "{}: {}",
                if cause.is_domain_error() {
                    "geometry domain error"
                } else {
                    "invalid geo configuration"
                },
                **cause
            ),
            Self::OperationValidation { context, .. } => {
                write!(f, "geometry domain error: {}", context.detail())
            }
            Self::MetricUnit(cause) => write!(f, "invalid geo configuration: {}", **cause),
            Self::MetricInput(cause) => write!(f, "geometry domain error: {}", **cause),
            Self::WktSyntax(cause) => write!(f, "malformed geometry literal: {}", **cause),
            Self::JsonSyntax(cause) => write!(f, "malformed geometry literal: {}", **cause),
            Self::PatternSyntax(cause) => write!(f, "malformed geometry literal: {cause}"),
            Self::Unsupported(msg) => write!(f, "unsupported GeoSPARQL operation: {msg}"),
            Self::Domain(msg) => write!(f, "geometry domain error: {msg}"),
            Self::GeometryDomain(cause) => write!(f, "geometry domain error: {}", **cause),
            Self::CarrierInput(cause) => {
                let prefix = if cause.is_configuration() {
                    "invalid geo configuration"
                } else {
                    "malformed geometry literal"
                };
                write!(f, "{prefix}: {}", **cause)
            }
            Self::CoordinateOutOfRange { axis, value } => {
                write!(
                    f,
                    "{axis} coordinate out of range: {}/{}",
                    number(value.numerator()),
                    number(value.denominator())
                )
            }
            Self::NonFiniteCoordinate { axis, bits } => {
                write!(f, "non-finite {axis} coordinate: binary64 {bits:#018x}")
            }
            Self::NonFiniteThreshold { bits } => {
                write!(f, "non-finite distance threshold: binary64 {bits:#018x}")
            }
            Self::NonNumericThreshold { .. } => {
                f.write_str("geometry domain error: a distance threshold must be numeric")
            }
            Self::UnregisteredCrs(iri) => write!(f, "unregistered coordinate reference: <{iri}>"),
            Self::MissingOperation { source, target } => {
                write!(
                    f,
                    "no explicit coordinate operation from <{source}> to <{target}>"
                )
            }
            Self::AmbiguousTransform { roots } => {
                write!(f, "ambiguous transform: {roots} admissible roots")
            }
            Self::PrecisionExhausted { bits } => write!(
                f,
                "precision exhausted at {bits} bits: required rounding remains uncertified"
            ),
            Self::WorkExhausted { limit } => write!(f, "geometry work exhausted at {limit} items"),
            Self::MemoryExhausted { limit } => {
                write!(f, "geometry workspace exhausted at {limit} bytes")
            }
            Self::NumericalScratch(error) => write!(f, "geometry numerical scratch: {error}"),
            Self::NumericalTaylorScratch(error) => write!(f, "geometry Taylor scratch: {error}"),
            Self::MissingNativeObserver { .. } => {
                f.write_str("invalid geo configuration: native entry requires its owned observer")
            }
            Self::MissingInverseMetadata => {
                f.write_str("geometry domain error: inverse metadata capture missing")
            }
            Self::DifferentialInput(_) => {
                f.write_str("numerical domain: invalid declared projection or source point")
            }
            Self::NativeMath(purrdf_xsd::math::MathError::Domain(reason)) => {
                write!(f, "geometry domain error: {reason}")
            }
            Self::NativeMath(error) => write!(f, "geometry numerical error: {error}"),
            Self::Storage(error) => write!(f, "GeoSPARQL native storage: {error}"),
            Self::OutputExhausted { limit } => {
                write!(f, "complete geometry output exceeds {limit} elements")
            }
            Self::ConvergenceExhausted { iterations } => write!(
                f,
                "convergence not certified within {iterations} iterations"
            ),
            Self::FloatEnvironment(error) => {
                write!(f, "geometry floating-point environment: {error}")
            }
            Self::NonPositiveEdgeLength(value) => write!(
                f,
                "maximum edge length must be positive: {}/{} metres",
                number(value.numerator()),
                number(value.denominator())
            ),
            Self::UnattainableEdgeLength { target, minimum } => write!(
                f,
                "maximum edge target {}/{} metres is below level-30 bound {}/{} metres",
                number(target.numerator()),
                number(target.denominator()),
                number(minimum.numerator()),
                number(minimum.denominator())
            ),
            Self::InvalidResolution(level) => {
                write!(f, "invalid cell level {level}: expected 0..=30")
            }
            Self::InvalidCellId(key) => write!(f, "invalid native cell key {key:#018x}"),
            Self::InvalidStoredLevel { cell, stored } => write!(
                f,
                "stored cell level {stored} cannot represent descendants of level {cell}"
            ),
            Self::InvalidAncestorLevel { cell, ancestor } => {
                write!(f, "level {ancestor} is not an ancestor of level {cell}")
            }
            Self::InvalidOutputLength { expected, actual } => {
                write!(f, "batch output length {actual}, expected {expected}")
            }
            Self::NegativePhysicalRadius(value) => write!(
                f,
                "negative physical radius: {}/{} metres",
                number(value.numerator()),
                number(value.denominator())
            ),
            Self::InvalidCoverLevels { min, max } => write!(
                f,
                "invalid cover levels {min}..={max}: expected 0 <= min <= max <= 30"
            ),
            Self::CoverCellsExhausted { limit } => {
                write!(f, "complete canonical cover exceeds {limit} cells")
            }
            Self::DuplicatePointKey(key) => write!(f, "duplicate point index key {key}"),
            Self::MissingHeight
            | Self::AmbiguousGeodesic
            | Self::InvalidEllipsoid(_)
            | Self::InvalidExecutionPolicy(_)
            | Self::InvalidCoverPolicy(_)
            | Self::InvalidPointIndexPolicy(_)
            | Self::InvalidNativeStorage(_)
            | Self::Cancelled
            | Self::ArithmeticOverflow(_)
            | Self::GridProfileMismatch
            | Self::RootHasNoParent
            | Self::LeafHasNoChildren => f.write_str(&self.detail()),
            Self::PointIndexReferenceMismatch => f.write_str(&self.detail()),
        }
    }
}

impl core::fmt::Display for GeoError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.fmt_with(formatter, |integer| integer)
    }
}

impl GeoError {
    /// Render the same diagnostic with native integer groups and UTF-8 output
    /// admitted before their actual allocation. Original input fields remain
    /// borrowed; every temporary group dies before its original grant shrinks.
    ///
    /// # Errors
    /// Preserves native integer, checked layout, allocator and account refusal.
    pub fn message_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<String, Self> {
        if let Self::Retained(original) = self {
            return original.original().message_with_memory(memory);
        }
        use purrdf_xsd::bigint::PreparedDecimalDigits;
        let inputs = match self {
            Self::CoordinateOutOfRange { value, .. }
            | Self::NonPositiveEdgeLength(value)
            | Self::NegativePhysicalRadius(value) => [
                Some(value.numerator()),
                Some(value.denominator()),
                None,
                None,
            ],
            Self::UnattainableEdgeLength { target, minimum } => [
                Some(target.numerator()),
                Some(target.denominator()),
                Some(minimum.numerator()),
                Some(minimum.denominator()),
            ],
            _ => core::array::from_fn(|_| None),
        };
        // The exhaustive number-bearing variants above use at most two ratios;
        // this stack array therefore covers the complete diagnostic law.
        memory.try_scope(|memory| {
            let mut prepared: [Option<PreparedDecimalDigits>; 4] = core::array::from_fn(|_| None);
            let mut retained = 0usize;
            for (slot, input) in prepared.iter_mut().zip(&inputs) {
                let Some(input) = input.as_ref() else {
                    continue;
                };
                let layout = input
                    .decimal_render_layout(0)
                    .map_err(Self::NumericalScratch)?;
                memory
                    .add_bytes(layout.temporary_bytes())
                    .map_err(Self::Storage)?;
                let digits = input
                    .prepare_decimal_digits()
                    .map_err(Self::NumericalScratch)?;
                let heap = digits.heap_bytes();
                let dead = layout
                    .temporary_bytes()
                    .checked_sub(heap)
                    .ok_or(Self::ArithmeticOverflow("native diagnostic decimal groups"))?;
                // Native quotient destinations have already died on return;
                // the prepared group payload remains covered in this slot.
                *slot = Some(digits);
                retained = retained.checked_add(heap).ok_or(Self::ArithmeticOverflow(
                    "native diagnostic group ownership",
                ))?;
                memory.release_bytes(dead).map_err(Self::Storage)?;
            }
            struct NativeMessage<'a, 'groups> {
                error: &'a GeoError,
                groups: &'groups [Option<PreparedDecimalDigits>; 4],
            }
            impl core::fmt::Display for NativeMessage<'_, '_> {
                fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    let mut next = 0usize;
                    self.error.fmt_with(formatter, |_| {
                        let digits = self.groups[next]
                            .as_ref()
                            .expect("every numerical diagnostic field was prepared in field order");
                        next += 1;
                        digits
                    })
                }
            }
            let rendered = memory
                .format(&NativeMessage {
                    error: self,
                    groups: &prepared,
                })
                .map_err(Self::Storage);
            drop(prepared);
            let refund = memory.release_bytes(retained).map_err(Self::Storage);
            match rendered {
                Ok(text) => refund.map(|()| text),
                Err(original) => Err(original),
            }
        })
    }
}

purrdf_lex::error_source!(GeoError => {
    Self::Retained(cause) => cause.original(),
    Self::SourceRead(cause) => cause,
    Self::Arity(cause) => cause,
    Self::NumericLiteral(cause) => cause,
    Self::ArgumentInput(cause) => &**cause,
    Self::Unsupported(cause) => cause,
    Self::IndexConfiguration(cause) => &**cause,
    Self::MatrixConfiguration(cause) => &**cause,
    Self::PatternSyntax(cause) => cause,
    Self::BindingConfiguration(cause) => &**cause,
    Self::InvalidEllipsoid(cause) => &**cause,
    Self::Vocabulary(cause) => &**cause,
    Self::GeometryDomain(cause) => &**cause,
    Self::MetricUnit(cause) => &**cause,
    Self::MetricInput(cause) => &**cause,
    Self::CarrierInput(cause) => &**cause,
    Self::CrsConfiguration(cause) | Self::CrsLiteral { cause, .. } => &**cause,
    Self::WktSyntax(cause) => &**cause,
    Self::JsonSyntax(cause) => &**cause,
    Self::GeometryShape(cause) => cause,
    Self::GeoJsonShape(cause) => &**cause,
    Self::GeoJsonWrite(cause) => &**cause,
    Self::IndexSerialization(cause) => &**cause,
    Self::IndexBinding(cause) => &**cause,
    Self::SourcePreparation(cause) => &**cause,
    Self::ContextConfiguration(cause) => &**cause,
    Self::TransverseInput(cause) => &**cause,
    Self::DifferentialInput(cause) => cause,
    Self::AffineInput(cause) => &**cause,
    Self::ProjectionInput(cause) => &**cause,
    Self::PolynomialInput(cause) => &**cause,
    Self::ModelInput(cause) => &**cause,
    Self::OperationValidation { context: _, cause } => cause,
    Self::Storage(cause) => cause,
    Self::NumericalScratch(cause) => cause,
    Self::NumericalTaylorScratch(cause) => cause,
    Self::NativeMath(cause) => cause,
    Self::FloatEnvironment(cause) => cause
});

purrdf_lex::variant_from!(GeoError {
    Storage(purrdf_lex::allocation::StorageError),
    NumericalScratch(purrdf_xsd::integer::LimbScratchError),
});
