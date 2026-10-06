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

/// Why a geometry, metric, coordinate operation or spatial index refused.
///
/// [`Self::is_expression_error`] defines the argument failures admitted by the
/// SPARQL adapter; every other variant remains fatal at that boundary.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GeoError {
    /// The input source refused a term or iterator read. No partial index is published.
    SourceRead(String),
    /// The call supplies the wrong number of arguments for the function named.
    Arity(String),
    /// The caller's wiring is unusable as written (an empty IRI, a vocabulary
    /// missing a term a registered function needs).
    Config(String),
    /// A geometry literal is not well-formed for its datatype.
    Literal(String),
    /// A spec-defined operation this crate does not implement. Always loud,
    /// never a default answer.
    Unsupported(String),
    /// Well-formed arguments on which the operation is undefined (mixed
    /// coordinate reference systems, a measure of an empty geometry).
    Domain(String),
    /// An original angular coordinate is outside its closed valid range.
    CoordinateOutOfRange {
        /// `longitude` or `latitude`.
        axis: &'static str,
        /// The exact input that was refused.
        value: crate::Rat,
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
    InvalidEllipsoid(&'static str),
    /// The execution policy cannot admit a complete operation.
    InvalidExecutionPolicy(&'static str),
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
    NonPositiveEdgeLength(crate::Rat),
    /// No admitted native hierarchy level reaches this physical target.
    UnattainableEdgeLength {
        /// The requested guaranteed upper edge length in metres.
        target: crate::Rat,
        /// The level-30 guaranteed upper bound in metres.
        minimum: Box<crate::Rat>,
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
    NegativePhysicalRadius(crate::Rat),
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

purrdf_lex::constructors! {
    impl GeoError {
        /// Construct a fatal operational source refusal.
        pub fn source_read(what) -> Self::SourceRead;
        /// A [`GeoError::Arity`] with `what` as its detail.
        pub fn arity(what) -> Self::Arity;

        /// A [`GeoError::Config`] with `what` as its detail.
        pub fn config(what) -> Self::Config;

        /// A [`GeoError::Literal`] with `what` as its detail.
        pub fn literal(what) -> Self::Literal;

        /// A [`GeoError::Unsupported`] with `what` as its detail.
        pub fn unsupported(what) -> Self::Unsupported;

        /// A [`GeoError::Domain`] with `what` as its detail.
        pub fn domain(what) -> Self::Domain;
    }
}

impl GeoError {
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
    pub const fn is_expression_error(&self) -> bool {
        matches!(
            self,
            Self::Domain(_)
                | Self::Literal(_)
                | Self::CoordinateOutOfRange { .. }
                | Self::NonFiniteCoordinate { .. }
                | Self::NonFiniteThreshold { .. }
                | Self::MissingHeight
                | Self::AmbiguousGeodesic
                | Self::AmbiguousTransform { .. }
                | Self::NegativePhysicalRadius(_)
        )
    }

    /// The detail message, without this error's own prefix.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Arity(msg)
            | Self::SourceRead(msg)
            | Self::Config(msg)
            | Self::Literal(msg)
            | Self::Unsupported(msg)
            | Self::Domain(msg) => msg,
            Self::UnregisteredCrs(msg) => msg,
            Self::InvalidEllipsoid(msg)
            | Self::InvalidExecutionPolicy(msg)
            | Self::ArithmeticOverflow(msg) => msg,
            Self::CoordinateOutOfRange { .. } => "original angular coordinate outside its range",
            Self::NonFiniteCoordinate { .. } => "non-finite angular coordinate",
            Self::NonFiniteThreshold { .. } => "non-finite distance threshold",
            Self::MissingOperation { .. } => "an explicit coordinate operation is required",
            Self::MissingHeight => "an actual height is required",
            Self::AmbiguousGeodesic => "endpoints do not select a unique geodesic arc",
            Self::AmbiguousTransform { .. } => "multiple admissible inverse roots",
            Self::PrecisionExhausted { .. } => "required rounding remains uncertified",
            Self::WorkExhausted { .. } => "admitted work exhausted",
            Self::MemoryExhausted { .. } => "admitted workspace exhausted",
            Self::NumericalScratch(_) => "admitted integer scratch refused a destination",
            Self::NumericalTaylorScratch(_) => "admitted Taylor scratch is already in use",
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
        }
    }
}

impl core::fmt::Display for GeoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SourceRead(msg) => write!(f, "geometry source read error: {msg}"),
            Self::Arity(msg) => write!(f, "wrong argument count: {msg}"),
            Self::Config(msg) => write!(f, "invalid geo configuration: {msg}"),
            Self::Literal(msg) => write!(f, "malformed geometry literal: {msg}"),
            Self::Unsupported(msg) => write!(f, "unsupported GeoSPARQL operation: {msg}"),
            Self::Domain(msg) => write!(f, "geometry domain error: {msg}"),
            Self::CoordinateOutOfRange { axis, value } => {
                write!(
                    f,
                    "{axis} coordinate out of range: {}/{}",
                    value.numerator(),
                    value.denominator()
                )
            }
            Self::NonFiniteCoordinate { axis, bits } => {
                write!(f, "non-finite {axis} coordinate: binary64 {bits:#018x}")
            }
            Self::NonFiniteThreshold { bits } => {
                write!(f, "non-finite distance threshold: binary64 {bits:#018x}")
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
                value.numerator(),
                value.denominator()
            ),
            Self::UnattainableEdgeLength { target, minimum } => write!(
                f,
                "maximum edge target {}/{} metres is below level-30 bound {}/{} metres",
                target.numerator(),
                target.denominator(),
                minimum.numerator(),
                minimum.denominator()
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
                value.numerator(),
                value.denominator()
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
            | Self::Cancelled
            | Self::ArithmeticOverflow(_)
            | Self::GridProfileMismatch
            | Self::RootHasNoParent
            | Self::LeafHasNoChildren => f.write_str(self.detail()),
            Self::PointIndexReferenceMismatch => f.write_str(self.detail()),
        }
    }
}

impl std::error::Error for GeoError {}
