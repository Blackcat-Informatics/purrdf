// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable standard carriers and explicit geographic reference registrations.

mod prepared_image;

#[derive(Clone, Copy)]
enum ImageTarget<'a> {
    Carrier(&'a Crs),
    Named(&'a Crs),
}

use std::sync::{LazyLock, OnceLock};

use purrdf_hash::{Domain, blake3, frame::frame_le_into, hex::Digest32};
use purrdf_iri::vocab::ogc;

use crate::{
    AxisOrder, Coord, Crs, ExecutionLimits, ExecutionPolicy, ExecutionPolicyId, GeoBindingId,
    GeoError, GeoVocab, GeographicReference, GeometryLiteral, LonLat, OperationChain, PointLaw,
    Rat, SemanticLawId,
};

const PROFILE_BINDING_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/carrier-bindings/v1");

/// Borrowed failure of the original carrier-resolution body. Owned fields
/// are created only by its caller's resident or retained native allocation home.
enum CarrierResolutionFailure<'a> {
    ConflictingOperation {
        first: &'a OperationBinding,
        second: &'a OperationBinding,
        first_chain: Digest32,
        second_chain: Digest32,
    },
    EmptyMetricSources,
    AmbiguousMetricSurface {
        first: &'a Crs,
        second: &'a Crs,
    },
    ImageEndpoints {
        reason: crate::ImageEndpointReason,
        expected_source: crate::operation::OperationReference,
        actual_source: crate::operation::OperationReference,
        expected_target: crate::operation::OperationReference,
        actual_target: crate::operation::OperationReference,
    },
    UnknownReference(&'a Crs),
    UnknownOperation(&'a Crs),
    MissingOperation(&'a Crs, &'a Crs),
    MultipleOperations {
        source: &'a Crs,
        target: &'a Crs,
        first: Digest32,
        second: Digest32,
    },
    ConflictingEndpoint {
        carrier: &'a Crs,
        first: crate::operation::OperationReference,
        second: crate::operation::OperationReference,
    },
    InvalidEndpoint {
        carrier: &'a Crs,
        expected: crate::operation::OperationReference,
        actual: crate::operation::OperationReference,
    },
}

impl purrdf_hash::error::StructuredError for CarrierResolutionFailure<'_> {
    purrdf_hash::error_visit! {
        ConflictingOperation { first, second, first_chain, second_chain } => "carrier-conflicting-operation" {
            "operation" => Record(&first.name), "first_source" => Record(&first.source), "first_target" => Record(&first.target),
            "first_chain" => Record(first_chain), "second_source" => Record(&second.source), "second_target" => Record(&second.target),
            "second_chain" => Record(second_chain)
        },
        EmptyMetricSources => "carrier-empty-metric-sources" {},
        AmbiguousMetricSurface { first, second } => "carrier-ambiguous-metric-surface" {
            "first" => Record(*first), "second" => Record(*second)
        },
        ImageEndpoints { reason, expected_source, actual_source, expected_target, actual_target } => "carrier-image-endpoints" {
            "reason" => Record(reason),
            "expected_source" => Record(expected_source), "actual_source" => Record(actual_source),
            "expected_target" => Record(expected_target), "actual_target" => Record(actual_target)
        },
        UnknownReference(reference) => "carrier-unknown-reference" { "reference" => Record(*reference) },
        UnknownOperation(operation) => "carrier-unknown-operation" { "operation" => Record(*operation) },
        MissingOperation(source, target) => "carrier-missing-operation" { "source" => Record(*source), "target" => Record(*target) },
        MultipleOperations { source, target, first, second } => "carrier-multiple-operations" {
            "source" => Record(*source), "target" => Record(*target), "first" => Record(first), "second" => Record(second)
        },
        ConflictingEndpoint { carrier, first, second } => "carrier-conflicting-endpoint" {
            "carrier" => Record(*carrier), "first" => Record(first), "second" => Record(second)
        },
        InvalidEndpoint { carrier, expected, actual } => "carrier-invalid-endpoint" {
            "carrier" => Record(*carrier), "expected" => Record(expected), "actual" => Record(actual)
        },
    }
}

impl CarrierResolutionFailure<'_> {
    fn with_memory(
        self,
        memory: &mut purrdf_lex::allocation::Memory<'_, dyn purrdf_lex::allocation::Admission + '_>,
    ) -> Result<GeoError, purrdf_lex::allocation::StorageError> {
        memory.try_scope(|memory| {
            let cause = match self {
                Self::ConflictingOperation {
                    first,
                    second,
                    first_chain,
                    second_chain,
                } => crate::BindingConfigError::ConflictingOperation {
                    operation: memory.string(first.name.as_str())?,
                    first_source: memory.string(first.source.as_str())?,
                    first_target: memory.string(first.target.as_str())?,
                    first_chain,
                    second_source: memory.string(second.source.as_str())?,
                    second_target: memory.string(second.target.as_str())?,
                    second_chain,
                },
                Self::EmptyMetricSources => crate::BindingConfigError::EmptyMetricSources,
                Self::AmbiguousMetricSurface { first, second } => {
                    crate::BindingConfigError::AmbiguousMetricSurface {
                        first: memory.string(first.as_str())?,
                        second: memory.string(second.as_str())?,
                    }
                }
                Self::ImageEndpoints {
                    reason,
                    expected_source,
                    actual_source,
                    expected_target,
                    actual_target,
                } => crate::BindingConfigError::ImageEndpoints {
                    reason,
                    expected_source,
                    actual_source,
                    expected_target,
                    actual_target,
                },
                Self::UnknownReference(crs) => {
                    return Ok(GeoError::UnregisteredCrs(memory.string(crs.as_str())?));
                }
                Self::MissingOperation(source, target) => {
                    return Ok(GeoError::MissingOperation {
                        source: memory.string(source.as_str())?,
                        target: memory.string(target.as_str())?,
                    });
                }
                Self::UnknownOperation(name) => crate::BindingConfigError::UnknownOperation {
                    operation: memory.string(name.as_str())?,
                },
                Self::InvalidEndpoint {
                    carrier,
                    expected,
                    actual,
                } => crate::BindingConfigError::InvalidEndpoint {
                    carrier: memory.string(carrier.as_str())?,
                    expected,
                    actual,
                },
                Self::MultipleOperations {
                    source,
                    target,
                    first,
                    second,
                } => crate::BindingConfigError::MultipleOperations {
                    source: memory.string(source.as_str())?,
                    target: memory.string(target.as_str())?,
                    first,
                    second,
                },
                Self::ConflictingEndpoint {
                    carrier,
                    first,
                    second,
                } => crate::BindingConfigError::ConflictingEndpoint {
                    carrier: memory.string(carrier.as_str())?,
                    first,
                    second,
                },
            };
            memory.boxed(cause).map(GeoError::BindingConfiguration)
        })
    }

    fn resident(self) -> GeoError {
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(
            &mut resident as &mut dyn purrdf_lex::allocation::Admission,
        );
        self.with_memory(&mut memory)
            .unwrap_or_else(GeoError::Storage)
    }

    fn retained(self, progress: &mut crate::context::WorkProgress<'_>) -> GeoError {
        progress.retained_error(|memory| self.with_memory(memory))
    }
}

/// The immutable standard reference profile shared by borrowed query contexts.
pub static STANDARD_PROFILE: GeoProfile = GeoProfile::STANDARD;
static STANDARD_VOCABULARY: GeoVocab = GeoVocab::STANDARD;
static CRS84_REFERENCE: LazyLock<GeographicReference> = LazyLock::new(GeographicReference::wgs84);

/// Borrow the immutable standard WGS84 longitude/latitude reference.
#[must_use]
pub fn standard_reference() -> &'static GeographicReference {
    &CRS84_REFERENCE
}

/// The one standard vocabulary shared by every native query and host adapter.
#[must_use]
pub fn standard_vocabulary() -> &'static GeoVocab {
    &STANDARD_VOCABULARY
}

/// Prepare the immutable standard vocabulary under configuration admission.
/// All original namespace/reference text and its constructor's temporary
/// copies are reserved before the existing lazy constructor can allocate.
/// Later invocations borrow this one prepared value.
///
/// # Errors
/// Refuses complete configuration work/storage before vocabulary construction.
pub fn standard_vocabulary_in_budget(
    budget: &mut crate::PreparationBudget,
) -> Result<&'static GeoVocab, GeoError> {
    let mut work = 1_u64;
    let mut storage = size_of::<GeoVocab>() as u64;
    for text in [
        ogc::geo::NS,
        ogc::geof::NS,
        ogc::CRS84,
        ogc::CRS84,
        ogc::uom::METRE,
        ogc::sf::NS,
    ] {
        let (items, bytes) =
            crate::carrier::InputAdmission::text_allowance(text.len() as u64, text.len() as u64)?;
        work = work
            .checked_add(items)
            .ok_or(GeoError::ArithmeticOverflow("standard vocabulary work"))?;
        storage = storage
            .checked_add(bytes)
            .ok_or(GeoError::ArithmeticOverflow("standard vocabulary storage"))?;
    }
    budget.retain(work, storage)?;
    Ok(standard_vocabulary())
}

/// Mathematical laws, reference bindings and admission carried independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GeoQueryIdentity {
    /// Correctly rounded true shortest point-distance law.
    pub distance_law: SemanticLawId,
    /// Canonical shortest-branch inverse metadata law.
    pub inverse_law: SemanticLawId,
    /// Frozen direct-coordinate output law.
    pub direct_law: SemanticLawId,
    /// Certified general geometry distance approximation law.
    pub geometry_distance_law: SemanticLawId,
    /// Certified greatest-dimension geographic length law.
    pub length_law: SemanticLawId,
    /// Certified geographic areal-boundary perimeter law.
    pub perimeter_law: SemanticLawId,
    /// Certified oriented geographic region-union area law.
    pub area_law: SemanticLawId,
    /// Certified signed integral on the original selected geodesic branch.
    pub geodesic_area_integral_law: SemanticLawId,
    /// Exact geographic set topology over the original prepared curve laws.
    pub topology_law: SemanticLawId,
    /// Certified complete continuous-image materialization output law.
    pub image_materialization_law: SemanticLawId,
    /// Certified point-offset materialization output law.
    pub point_buffer_materialization_law: SemanticLawId,
    /// Certified complete-source offset materialization output law.
    pub buffer_materialization_law: SemanticLawId,
    /// Certified global point-offset carrier materialization law.
    pub global_point_buffer_materialization_law: SemanticLawId,
    /// Certified global complete-source offset carrier materialization law.
    pub global_buffer_materialization_law: SemanticLawId,
    /// Immutable standard and explicit caller reference bindings.
    pub binding: GeoBindingId,
    /// Admitted work, precision, output and memory policy.
    pub policy: ExecutionPolicyId,
}

impl GeoQueryIdentity {
    /// The fixed canonical frame contains sixteen 32-byte identities.
    pub const FRAMED_BYTES: usize = 16 * (size_of::<u64>() + 32);

    /// Canonical framing for caches and query provenance; no implementation receipt.
    pub fn append_to(&self, output: &mut Vec<u8>) {
        output.reserve(Self::FRAMED_BYTES);
        self.framed_parts(|part| output.extend_from_slice(part));
    }

    /// The same canonical frame in caller-independent fixed storage.
    #[must_use]
    pub fn framed_bytes(&self) -> [u8; Self::FRAMED_BYTES] {
        let mut output = [0; Self::FRAMED_BYTES];
        let mut offset = 0;
        self.framed_parts(|part| {
            let end = offset + part.len();
            output[offset..end].copy_from_slice(part);
            offset = end;
        });
        debug_assert_eq!(offset, Self::FRAMED_BYTES);
        output
    }

    fn framed_parts(&self, mut append: impl FnMut(&[u8])) {
        for digest in [
            self.distance_law.digest(),
            self.inverse_law.digest(),
            self.direct_law.digest(),
            self.geometry_distance_law.digest(),
            self.length_law.digest(),
            self.perimeter_law.digest(),
            self.area_law.digest(),
            self.geodesic_area_integral_law.digest(),
            self.topology_law.digest(),
            self.image_materialization_law.digest(),
            self.point_buffer_materialization_law.digest(),
            self.buffer_materialization_law.digest(),
            self.global_point_buffer_materialization_law.digest(),
            self.global_buffer_materialization_law.digest(),
            self.binding.digest(),
            self.policy.digest(),
        ] {
            purrdf_hash::frame::frame_le_with(digest.as_bytes(), &mut append);
        }
    }
}

/// An explicit carrier reference declaration; an ellipsoid alone changes no datum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceBinding {
    crs: Crs,
    reference: GeographicReference,
}

impl ReferenceBinding {
    /// The exact carrier IRI.
    #[must_use]
    pub const fn crs(&self) -> &Crs {
        &self.crs
    }

    /// The declared datum, ellipsoid and source axis order.
    #[must_use]
    pub const fn reference(&self) -> &GeographicReference {
        &self.reference
    }
}

/// Geographic carrier registrations and independently admitted execution limits.
///
/// CRS84 is always WGS84 longitude/latitude. Every other reference must be
/// registered explicitly, including EPSG:4326. No datum operation is inferred.
#[derive(Clone)]
pub struct GeoProfile {
    references: crate::PreparedValues<ReferenceBinding>,
    operations: crate::PreparedValues<OperationBinding>,
    units: crate::PreparedValues<LinearUnitBinding>,
    limits: ExecutionLimits,
    // Derived immutable metadata. Source registration invalidates this value;
    // policy remains a separate component of the query identity.
    binding: OnceLock<GeoBindingId>,
}

// Derived cache state is excluded so cold and warm profiles preserve the
// original source value's identical Debug spelling.
impl core::fmt::Debug for GeoProfile {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use purrdf_lex::walk::{Tok, WorkList, write_debug};
        write_debug(
            formatter,
            self,
            |profile, output: &mut WorkList<Tok<&Self, &dyn core::fmt::Debug>, 16>| {
                output.extend([
                    Tok::Struct("GeoProfile"),
                    Tok::Field("references"),
                    Tok::Leaf(&profile.references as &dyn core::fmt::Debug),
                    Tok::Field("operations"),
                    Tok::Leaf(&profile.operations),
                    Tok::Field("units"),
                    Tok::Leaf(&profile.units),
                    Tok::Field("limits"),
                    Tok::Leaf(&profile.limits),
                    Tok::EndStruct,
                ]);
            },
        )
    }
}

impl PartialEq for GeoProfile {
    fn eq(&self, other: &Self) -> bool {
        self.references == other.references
            && self.operations == other.operations
            && self.units == other.units
            && self.limits == other.limits
    }
}
impl Eq for GeoProfile {}

purrdf_hash::default_from_new!(GeoProfile => standard);

impl GeoProfile {
    /// Immutable standard behavior with no additional reference registrations.
    #[expect(
        clippy::declare_interior_mutable_const,
        reason = "The existing value constant constructs an independent empty derived cache for each profile; no mutable state is shared through the constant"
    )]
    pub const STANDARD: Self = Self {
        references: crate::PreparedValues::EMPTY,
        operations: crate::PreparedValues::EMPTY,
        units: crate::PreparedValues::EMPTY,
        limits: ExecutionLimits::GEOMETRY,
        binding: OnceLock::new(),
    };

    /// Standard WGS84 longitude/latitude carriers and frozen geometry limits.
    #[must_use]
    pub const fn standard() -> Self {
        Self::STANDARD
    }

    /// Register a reference under its exact carrier IRI.
    ///
    /// Repeating the identical declaration coalesces. CRS84 can only repeat
    /// WGS84 longitude/latitude. EPSG:4326 requires WGS84 latitude/longitude;
    /// no axis order is inferred from a name, a numeric range or a dataset.
    ///
    /// # Errors
    ///
    /// Refuses conflicting, protected-carrier or invalid EPSG:4326 declarations.
    pub fn register_reference(
        &mut self,
        crs: Crs,
        reference: GeographicReference,
    ) -> Result<(), GeoError> {
        self.register_reference_in_budget(
            crs,
            reference,
            &mut crate::PreparationBudget::new(ExecutionPolicy::geometry()),
        )
    }

    /// Admit exact reference equality and cold binding IDs before registration.
    /// # Errors
    /// Refuses conflicting carriers and cumulative configuration limits.
    pub fn register_reference_in_budget(
        &mut self,
        crs: Crs,
        reference: GeographicReference,
        budget: &mut crate::PreparationBudget,
    ) -> Result<(), GeoError> {
        self.register_reference_observed(crs, reference, budget, None)
    }

    /// Register the same exact declaration through its original physical owner.
    /// New table/control allocations and diagnostics are admitted before birth.
    ///
    /// # Errors
    /// Adds original observer/allocator refusal to [`Self::register_reference_in_budget`].
    pub fn register_reference_in_budget_metered(
        &mut self,
        crs: Crs,
        reference: GeographicReference,
        budget: &mut crate::PreparationBudget,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.register_reference_observed(crs, reference, budget, Some(observer))
    }

    fn register_reference_observed(
        &mut self,
        crs: Crs,
        reference: GeographicReference,
        budget: &mut crate::PreparationBudget,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        let mut cost = crate::numerical::reference_copy_cost(&reference)?;
        for value in [
            reference.ellipsoid().semimajor(),
            reference.ellipsoid().inverse_flattening(),
        ] {
            cost = cost
                .followed_by(crate::numerical::rational_field_cost(value)?.0)
                .ok_or(GeoError::ArithmeticOverflow("reference registration work"))?;
        }
        let bytes = reference
            .ellipsoid()
            .retained_limb_bytes()
            .checked_add(crs.retained_text_bytes() as u64)
            .and_then(|bytes| bytes.checked_add(size_of::<ReferenceBinding>() as u64))
            .ok_or(GeoError::ArithmeticOverflow(
                "reference registration storage",
            ))?;
        self.registration_in_budget::<_, ReferenceBinding>(
            bytes,
            cost,
            self.references.len(),
            budget,
            |profile, values, progress| {
                profile.register_reference_admitted(crs, reference, values, progress)
            },
            observer,
        )
    }

    fn register_reference_admitted(
        &mut self,
        crs: Crs,
        reference: GeographicReference,
        values: &crate::numerical::ExactValues<'_>,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        if reference.cached_identity().is_none() {
            reference.cache_identity(crate::numerical::reference_identity_using(
                &reference, values,
            )?);
        }
        if crs.as_str() == ogc::CRS84 {
            return if reference == *CRS84_REFERENCE {
                Ok(())
            } else {
                Err(crate::BindingConfigError::reference_registration(
                    &crs,
                    &reference,
                    &CRS84_REFERENCE,
                    crate::ReferenceRegistrationReason::Crs84,
                    progress,
                ))
            };
        }
        if crs.as_str() == ogc::EPSG4326
            && reference != CRS84_REFERENCE.clone().with_axes(AxisOrder::LatLon)
        {
            return Err(crate::BindingConfigError::reference_registration(
                &crs,
                &reference,
                &CRS84_REFERENCE.clone().with_axes(AxisOrder::LatLon),
                crate::ReferenceRegistrationReason::Epsg4326,
                progress,
            ));
        }
        for operation in &self.operations {
            for (carrier, endpoint) in [
                (
                    &operation.source,
                    operation
                        .chain
                        .operations()
                        .first()
                        .expect("compiled chain")
                        .source(),
                ),
                (
                    &operation.target,
                    operation
                        .chain
                        .operations()
                        .last()
                        .expect("compiled chain")
                        .target(),
                ),
            ] {
                if carrier == &crs {
                    validate_endpoint_resolution(carrier, &reference, endpoint)
                        .map_err(|failure| failure.retained(progress))?;
                }
            }
        }
        match self
            .references
            .binary_search_by(|entry| entry.crs.cmp(&crs))
        {
            Ok(index) if self.references[index].reference == reference => Ok(()),
            Ok(index) => Err(crate::BindingConfigError::reference_registration(
                &crs,
                &reference,
                &self.references[index].reference,
                crate::ReferenceRegistrationReason::ConflictingRegistration,
                progress,
            )),
            Err(index) => {
                self.references = self.references.inserted_with(
                    index,
                    ReferenceBinding { crs, reference },
                    progress,
                    |row| Ok(row.clone()),
                )?;
                self.binding = OnceLock::new();
                Ok(())
            }
        }
    }

    /// Replace execution admission without changing a completed-output law.
    ///
    /// # Errors
    ///
    /// Refuses invalid limits before computation.
    pub fn with_limits(mut self, limits: ExecutionLimits) -> Result<Self, GeoError> {
        ExecutionPolicy::new(limits)?;
        self.limits = limits;
        Ok(self)
    }

    /// The independently identified admission policy.
    #[must_use]
    pub fn policy(&self) -> ExecutionPolicy {
        ExecutionPolicy::new(self.limits).expect("construction validated this profile's limits")
    }

    /// Explicit registrations, in exact IRI order.
    #[must_use]
    pub fn references(&self) -> &[ReferenceBinding] {
        &self.references
    }

    /// Register an exact positive number of metres per linear output unit.
    /// The official metre is immutable; repeating an identical unit coalesces.
    /// Areal metrics use the square of the same declared factor.
    ///
    /// # Errors
    ///
    /// Refuses nonpositive factors and conflicting unit declarations.
    pub fn register_linear_unit(
        &mut self,
        unit: Crs,
        metres_per_unit: Rat,
    ) -> Result<(), GeoError> {
        self.register_linear_unit_in_budget(
            unit,
            metres_per_unit,
            &mut crate::PreparationBudget::new(ExecutionPolicy::geometry()),
        )
    }

    /// Admit the original factor and unit registration within one configuration budget.
    /// # Errors
    /// Refuses factor conflicts and work/storage limits before exact equality.
    pub fn register_linear_unit_in_budget(
        &mut self,
        unit: Crs,
        metres_per_unit: Rat,
        budget: &mut crate::PreparationBudget,
    ) -> Result<(), GeoError> {
        self.register_linear_unit_observed(unit, metres_per_unit, budget, None)
    }

    /// Register an exact unit while retaining every original native table owner.
    ///
    /// # Errors
    /// Adds physical observer refusal to [`Self::register_linear_unit_in_budget`].
    pub fn register_linear_unit_in_budget_metered(
        &mut self,
        unit: Crs,
        metres_per_unit: Rat,
        budget: &mut crate::PreparationBudget,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.register_linear_unit_observed(unit, metres_per_unit, budget, Some(observer))
    }

    fn register_linear_unit_observed(
        &mut self,
        unit: Crs,
        metres_per_unit: Rat,
        budget: &mut crate::PreparationBudget,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        let cost = crate::numerical::rational_field_cost(&metres_per_unit)?.0;
        let bytes = (metres_per_unit.allocated_bytes() as u64)
            .checked_add(unit.retained_text_bytes() as u64)
            .and_then(|bytes| bytes.checked_add(size_of::<LinearUnitBinding>() as u64))
            .ok_or(GeoError::ArithmeticOverflow("unit registration storage"))?;
        self.registration_in_budget::<_, LinearUnitBinding>(
            bytes,
            cost,
            self.units.len(),
            budget,
            |profile, _, progress| {
                profile.register_linear_unit_admitted(unit, metres_per_unit, progress)
            },
            observer,
        )
    }

    fn register_linear_unit_admitted(
        &mut self,
        unit: Crs,
        metres_per_unit: Rat,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        if metres_per_unit.signum() <= 0 {
            return Err(crate::BindingConfigError::unit_factor(
                &unit,
                &metres_per_unit,
                None,
                crate::UnitFactorReason::NonPositive,
                progress,
            ));
        }
        if unit.as_str() == ogc::uom::METRE {
            return if metres_per_unit == Rat::one() {
                Ok(())
            } else {
                Err(crate::BindingConfigError::unit_factor(
                    &unit,
                    &metres_per_unit,
                    Some(&Rat::one()),
                    crate::UnitFactorReason::OfficialMetre,
                    progress,
                ))
            };
        }
        match self.units.binary_search_by(|entry| entry.unit.cmp(&unit)) {
            Ok(index) if self.units[index].metres_per_unit == metres_per_unit => Ok(()),
            Ok(index) => Err(crate::BindingConfigError::unit_factor(
                &unit,
                &metres_per_unit,
                Some(&self.units[index].metres_per_unit),
                crate::UnitFactorReason::ConflictingRegistration,
                progress,
            )),
            Err(index) => {
                self.units = self.units.inserted_with(
                    index,
                    LinearUnitBinding {
                        unit,
                        metres_per_unit,
                    },
                    progress,
                    |row| Ok(row.clone()),
                )?;
                self.binding = OnceLock::new();
                Ok(())
            }
        }
    }

    /// Explicit linear output-unit declarations in exact IRI order.
    #[must_use]
    pub fn linear_units(&self) -> &[LinearUnitBinding] {
        &self.units
    }

    /// Resolve an exact linear output-unit factor without guessing from its name.
    ///
    /// # Errors
    ///
    /// Refuses every undeclared unit except the immutable official metre.
    pub fn metres_per_unit(&self, unit: &Crs) -> Result<Rat, GeoError> {
        self.metres_per_unit_value(unit)
            .map(std::borrow::Cow::into_owned)
    }

    /// Resolve a registered factor by reference, retaining the exact declared
    /// limbs for admitted arithmetic instead of cloning them before admission.
    /// The immutable official metre uses the small owned exact value one.
    ///
    /// # Errors
    /// Refuses every undeclared unit except the official metre.
    pub fn metres_per_unit_value(&self, unit: &Crs) -> Result<std::borrow::Cow<'_, Rat>, GeoError> {
        if unit.as_str() == ogc::uom::METRE {
            return Ok(std::borrow::Cow::Owned(Rat::one()));
        }
        self.units
            .binary_search_by(|entry| entry.unit.cmp(unit))
            .map(|index| std::borrow::Cow::Borrowed(&self.units[index].metres_per_unit))
            .map_err(|_| crate::VocabularyError::unregistered_output_unit(unit))
    }

    /// Register a compiled operation chain under an exact caller-supplied IRI.
    /// Source and target carrier declarations are explicit; no conversion is inferred.
    ///
    /// # Errors
    ///
    /// Refuses conflicting names and a geographic endpoint whose explicit
    /// realization, degree units or axis declaration disagrees with its carrier.
    pub fn register_operation(
        &mut self,
        name: Crs,
        source: Crs,
        target: Crs,
        chain: OperationChain,
    ) -> Result<(), GeoError> {
        self.register_operation_in_budget(
            name,
            source,
            target,
            chain,
            &mut crate::PreparationBudget::new(ExecutionPolicy::geometry()),
        )
    }

    /// Admit chain storage, binding checks and table insertion cumulatively.
    /// # Errors
    /// Refuses declaration conflicts and complete work/storage exhaustion.
    pub fn register_operation_in_budget(
        &mut self,
        name: Crs,
        source: Crs,
        target: Crs,
        chain: OperationChain,
        budget: &mut crate::PreparationBudget,
    ) -> Result<(), GeoError> {
        self.register_operation_observed((name, source, target, chain), budget, None)
    }

    /// Register a compiled chain with admitted immutable native table growth.
    ///
    /// # Errors
    /// Adds physical refusal to [`Self::register_operation_in_budget`].
    pub fn register_operation_in_budget_metered(
        &mut self,
        name: Crs,
        source: Crs,
        target: Crs,
        chain: OperationChain,
        budget: &mut crate::PreparationBudget,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.register_operation_observed((name, source, target, chain), budget, Some(observer))
    }

    fn register_operation_observed(
        &mut self,
        (name, source, target, chain): (Crs, Crs, Crs, OperationChain),
        budget: &mut crate::PreparationBudget,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        budget.retain(chain.storage_walk_work()?, 0)?;
        let bytes = chain
            .retained_workspace_bytes()?
            .checked_add(name.retained_text_bytes() as u64)
            .and_then(|bytes| bytes.checked_add(source.retained_text_bytes() as u64))
            .and_then(|bytes| bytes.checked_add(target.retained_text_bytes() as u64))
            .and_then(|bytes| bytes.checked_add(size_of::<OperationBinding>() as u64))
            .ok_or(GeoError::ArithmeticOverflow(
                "operation registration storage",
            ))?;
        let cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            purrdf_xsd::integer::ExactOperation::Linear,
            1,
            1,
        )
        .ok_or(GeoError::ArithmeticOverflow("operation registration work"))?;
        self.registration_in_budget::<_, OperationBinding>(
            bytes,
            cost,
            self.operations.len(),
            budget,
            |profile, _, progress| {
                profile.register_operation_admitted(name, source, target, chain, progress)
            },
            observer,
        )
    }

    fn register_operation_admitted(
        &mut self,
        name: Crs,
        source: Crs,
        target: Crs,
        chain: OperationChain,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        for (carrier, endpoint) in [
            (
                &source,
                chain
                    .operations()
                    .first()
                    .expect("a compiled chain is nonempty")
                    .source(),
            ),
            (
                &target,
                chain
                    .operations()
                    .last()
                    .expect("a compiled chain is nonempty")
                    .target(),
            ),
        ] {
            if let Some(reference) = self.find_reference(carrier) {
                validate_endpoint_resolution(carrier, reference, endpoint)
                    .map_err(|failure| failure.retained(progress))?;
            }
        }
        let binding = OperationBinding {
            name,
            source,
            target,
            chain,
        };
        match self
            .operations
            .binary_search_by(|entry| entry.name.cmp(&binding.name))
        {
            Ok(index) if self.operations[index] == binding => Ok(()),
            Ok(index) => Err(CarrierResolutionFailure::ConflictingOperation {
                first: &self.operations[index],
                second: &binding,
                first_chain: self.operations[index].chain.id(),
                second_chain: binding.chain.id(),
            }
            .retained(progress)),
            Err(index) => {
                self.operations =
                    self.operations
                        .inserted_with(index, binding, progress, |row| Ok(row.clone()))?;
                self.binding = OnceLock::new();
                Ok(())
            }
        }
    }

    /// Compiled operation registrations in exact dispatch-name order.
    #[must_use]
    pub fn operations(&self) -> &[OperationBinding] {
        &self.operations
    }

    /// Resolve one explicit named operation, including its carrier endpoints.
    ///
    /// # Errors
    ///
    /// Refuses a missing operation; a name never selects an inferred datum change.
    pub fn operation(&self, name: &Crs) -> Result<&OperationBinding, GeoError> {
        self.find_operation(name)
            .ok_or_else(|| CarrierResolutionFailure::UnknownOperation(name).resident())
    }

    fn find_operation(&self, name: &Crs) -> Option<&OperationBinding> {
        self.operations
            .binary_search_by(|entry| entry.name.cmp(name))
            .map(|index| &self.operations[index])
            .ok()
    }

    pub(crate) fn operation_with_progress<'a>(
        &'a self,
        name: &Crs,
        context: &mut crate::MetricContext,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<&'a OperationBinding, GeoError> {
        let cost = carrier_lookup_cost(
            &[name],
            self.operations.iter().map(|entry| &entry.name),
            self.operations.len(),
            context,
            progress,
        )?;
        progress
            .exact(context, cost, || Ok(self.find_operation(name)))?
            .ok_or_else(|| CarrierResolutionFailure::UnknownOperation(name).retained(progress))
    }

    pub(crate) fn operation_between_with_progress<'a>(
        &'a self,
        source: &Crs,
        target: &Crs,
        context: &mut crate::MetricContext,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<&'a OperationChain, GeoError> {
        let cost = carrier_lookup_cost(
            &[source, target],
            self.operations
                .iter()
                .flat_map(|entry| [&entry.source, &entry.target]),
            self.operations
                .len()
                .checked_mul(2)
                .ok_or(GeoError::ArithmeticOverflow("operation lookup count"))?,
            context,
            progress,
        )?;
        progress
            .exact(context, cost, || {
                Ok(self.operation_between_resolution(source, target))
            })?
            .map_err(|failure| failure.retained(progress))
    }

    fn image_target_with_progress<'a>(
        &'a self,
        source: &Crs,
        target: ImageTarget<'a>,
        context: &mut crate::MetricContext,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<(&'a Crs, Option<&'a OperationChain>), GeoError> {
        match target {
            ImageTarget::Carrier(target) => Ok((target, None)),
            ImageTarget::Named(name) => {
                let binding = self.operation_with_progress(name, context, progress)?;
                let cost = carrier_lookup_cost(
                    &[source, binding.target()],
                    core::iter::once(binding.source()),
                    1,
                    context,
                    progress,
                )?;
                if !progress.exact(context, cost, || Ok(source == binding.source()))? {
                    return Err(CarrierResolutionFailure::MissingOperation(
                        source,
                        binding.target(),
                    )
                    .retained(progress));
                }
                Ok((binding.target(), Some(binding.chain())))
            }
        }
    }

    /// Resolve a unique explicit chain between exact carrier IRIs.
    ///
    /// # Errors
    ///
    /// Refuses absent chains and multiple distinct chains rather than guessing.
    pub fn operation_between(
        &self,
        source: &Crs,
        target: &Crs,
    ) -> Result<&OperationChain, GeoError> {
        self.operation_between_resolution(source, target)
            .map_err(CarrierResolutionFailure::resident)
    }

    #[allow(
        clippy::result_large_err,
        reason = "The pre-admission refusal borrows original bindings and carries exact endpoint identities inline; boxing it would allocate before its caller admits the diagnostic"
    )]
    fn operation_between_resolution<'a, 'b>(
        &'a self,
        source: &'b Crs,
        target: &'b Crs,
    ) -> Result<&'a OperationChain, CarrierResolutionFailure<'b>> {
        let mut found: Option<&OperationChain> = None;
        for binding in &self.operations {
            if &binding.source == source && &binding.target == target {
                if let Some(previous) = found
                    && previous.id() != binding.chain.id()
                {
                    return Err(CarrierResolutionFailure::MultipleOperations {
                        source,
                        target,
                        first: previous.id(),
                        second: binding.chain.id(),
                    });
                }
                found = Some(&binding.chain);
            }
        }
        found.ok_or(CarrierResolutionFailure::MissingOperation(source, target))
    }

    /// Resolve a common physical metric surface from actual declarations.
    /// Already equivalent geographic sources retain that surface. Otherwise
    /// every source must have an explicit chain to the same declared target;
    /// different reachable target surfaces are an ambiguity, never a guess.
    ///
    /// # Errors
    /// Refuses empty input, missing carriers/chains and ambiguous target surfaces.
    pub fn metric_reference(&self, sources: &[&Crs]) -> Result<Crs, GeoError> {
        let Some(first) = sources.first() else {
            return Err(CarrierResolutionFailure::EmptyMetricSources.resident());
        };
        for source in sources {
            self.operation_reference(source)?;
        }
        if let Ok(surface) = self.reference(first)
            && sources.iter().all(|source| {
                self.reference(source)
                    .is_ok_and(|reference| reference.same_surface(surface))
            })
        {
            return if CRS84_REFERENCE.same_surface(surface) {
                Crs::new(ogc::CRS84)
            } else {
                Ok((*sources.iter().min().expect("nonempty sources")).clone())
            };
        }
        let standard = Crs::new(ogc::CRS84)?;
        let mut selected: Option<&Crs> = None;
        for target in
            std::iter::once(&standard).chain(self.references.iter().map(ReferenceBinding::crs))
        {
            let reference = self.reference(target)?;
            let mut reachable = true;
            for source in sources {
                if self
                    .reference(source)
                    .is_ok_and(|source| source.same_surface(reference))
                {
                    continue;
                }
                match self.operation_between(source, target) {
                    Ok(_) => {}
                    Err(GeoError::MissingOperation { .. }) => {
                        reachable = false;
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }
            if reachable {
                if let Some(previous) = selected {
                    if !self.reference(previous)?.same_surface(reference) {
                        return Err(CarrierResolutionFailure::AmbiguousMetricSurface {
                            first: previous,
                            second: target,
                        }
                        .resident());
                    }
                    if target < previous {
                        selected = Some(target);
                    }
                } else {
                    selected = Some(target);
                }
            }
        }
        selected.cloned().ok_or_else(|| GeoError::MissingOperation {
            source: first.as_str().to_owned(),
            target: sources.get(1).unwrap_or(first).as_str().to_owned(),
        })
    }

    /// Resolve a registered reference or the immutable standard CRS84 binding.
    ///
    /// # Errors
    ///
    /// Returns typed [`GeoError::UnregisteredCrs`] for every undeclared reference.
    pub fn reference(&self, crs: &Crs) -> Result<&GeographicReference, GeoError> {
        self.find_reference(crs)
            .ok_or_else(|| CarrierResolutionFailure::UnknownReference(crs).resident())
    }

    fn find_reference(&self, crs: &Crs) -> Option<&GeographicReference> {
        if crs.as_str() == ogc::CRS84 {
            return Some(&CRS84_REFERENCE);
        }
        self.references
            .binary_search_by(|entry| entry.crs.cmp(crs))
            .map(|index| &self.references[index].reference)
            .ok()
    }

    /// The same borrowed lookup at source preparation's pre-context boundary.
    pub(crate) fn reference_retained<'a>(
        &'a self,
        crs: &Crs,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<&'a GeographicReference, GeoError> {
        self.find_reference(crs)
            .ok_or_else(|| CarrierResolutionFailure::UnknownReference(crs).retained(progress))
    }

    /// Govern the original sorted binding lookup before text comparisons or
    /// an unknown-carrier error copy. Reference identity rendering is admitted
    /// separately by the caller through the shared numerical identity home.
    pub(crate) fn reference_with_progress<'a>(
        &'a self,
        crs: &Crs,
        context: &mut crate::MetricContext,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<&'a GeographicReference, GeoError> {
        let cost = carrier_lookup_cost(
            &[crs],
            self.references.iter().map(|entry| &entry.crs),
            self.references.len(),
            context,
            progress,
        )?;
        progress
            .exact(context, cost, || Ok(self.find_reference(crs)))?
            .ok_or_else(|| CarrierResolutionFailure::UnknownReference(crs).retained(progress))
    }

    /// Resolve an explicitly registered operation carrier, including projected metre axes.
    ///
    /// # Errors
    /// Refuses an undeclared carrier, ambiguous bindings, and degree carriers
    /// without an actual geographic registration. EPSG:4326 keeps its explicit rule.
    pub fn operation_reference(
        &self,
        crs: &Crs,
    ) -> Result<crate::operation::OperationReference, GeoError> {
        if let Some(reference) = self.find_reference(crs) {
            return Ok(geographic_operation_reference(reference, reference.id()));
        }
        self.projected_operation_reference(crs)
    }

    pub(crate) fn operation_reference_with_progress(
        &self,
        crs: &Crs,
        context: &mut crate::MetricContext,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<crate::operation::OperationReference, GeoError> {
        let cost = carrier_lookup_cost(
            &[crs],
            self.references.iter().map(|entry| &entry.crs),
            self.references.len(),
            context,
            progress,
        )?;
        if let Some(reference) = progress.exact(context, cost, || Ok(self.find_reference(crs)))? {
            let identity =
                crate::numerical::reference_identity(context, progress, Some(reference))?;
            return Ok(geographic_operation_reference(reference, identity));
        }
        let cost = carrier_lookup_cost(
            &[crs],
            self.operations
                .iter()
                .flat_map(|entry| [&entry.source, &entry.target]),
            self.operations
                .len()
                .checked_mul(2)
                .ok_or(GeoError::ArithmeticOverflow("operation carrier count"))?,
            context,
            progress,
        )?;
        progress
            .exact(context, cost, || {
                Ok(self.projected_operation_reference_resolution(crs))
            })?
            .map_err(|failure| failure.retained(progress))
    }

    fn projected_operation_reference(
        &self,
        crs: &Crs,
    ) -> Result<crate::operation::OperationReference, GeoError> {
        self.projected_operation_reference_resolution(crs)
            .map_err(CarrierResolutionFailure::resident)
    }

    #[allow(
        clippy::result_large_err,
        reason = "The pre-admission refusal borrows original bindings and carries exact endpoint identities inline; boxing it would allocate before its caller admits the diagnostic"
    )]
    fn projected_operation_reference_resolution<'a>(
        &self,
        crs: &'a Crs,
    ) -> Result<crate::operation::OperationReference, CarrierResolutionFailure<'a>> {
        use crate::operation::CoordinateUnit;
        if crs.as_str() == ogc::EPSG4326 {
            return Err(CarrierResolutionFailure::UnknownReference(crs));
        }
        let mut declared = None;
        for binding in &self.operations {
            for (carrier, endpoint) in [
                (
                    &binding.source,
                    binding
                        .chain
                        .operations()
                        .first()
                        .expect("compiled chain")
                        .source(),
                ),
                (
                    &binding.target,
                    binding
                        .chain
                        .operations()
                        .last()
                        .expect("compiled chain")
                        .target(),
                ),
            ] {
                if carrier != crs {
                    continue;
                }
                if endpoint.unit == CoordinateUnit::Degrees {
                    return Err(CarrierResolutionFailure::UnknownReference(crs));
                }
                if let Some(previous) = declared
                    && previous != endpoint
                {
                    return Err(CarrierResolutionFailure::ConflictingEndpoint {
                        carrier: crs,
                        first: previous,
                        second: endpoint,
                    });
                }
                declared = Some(endpoint);
            }
        }
        declared.ok_or(CarrierResolutionFailure::UnknownReference(crs))
    }

    /// Materialize the complete continuous image through a registered actual chain.
    ///
    /// # Errors
    /// Preserves reference, domain, topology, certificate and resource refusals.
    pub fn transform_literal(
        &self,
        source: &GeometryLiteral,
        target: &Crs,
        epoch: Option<&Rat>,
        context: &mut crate::MetricContext,
    ) -> Result<crate::operation::GeometryImage, GeoError> {
        self.transform_literal_inner(source, ImageTarget::Carrier(target), epoch, context, None)
    }

    /// Complete continuous carrier transformation with bounded governor polling.
    ///
    /// # Errors
    /// Also propagates cancellation without a successful partial geometry.
    pub fn transform_literal_metered(
        &self,
        source: &GeometryLiteral,
        target: &Crs,
        epoch: Option<&Rat>,
        context: &mut crate::MetricContext,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<crate::operation::GeometryImage, GeoError> {
        self.transform_literal_inner(
            source,
            ImageTarget::Carrier(target),
            epoch,
            context,
            Some(observer),
        )
    }
    /// Materialize the complete image through one explicitly named registered chain.
    ///
    /// # Errors
    /// Refuses a source-carrier mismatch and all native image/certificate errors.
    pub fn transform_literal_named(
        &self,
        name: &Crs,
        source: &GeometryLiteral,
        epoch: Option<&Rat>,
        context: &mut crate::MetricContext,
    ) -> Result<crate::operation::GeometryImage, GeoError> {
        self.transform_literal_inner(source, ImageTarget::Named(name), epoch, context, None)
    }

    fn transform_literal_inner(
        &self,
        source: &GeometryLiteral,
        target: ImageTarget<'_>,
        epoch: Option<&Rat>,
        context: &mut crate::MetricContext,
        mut observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<crate::operation::GeometryImage, GeoError> {
        use crate::operation::{CoordinateUnit, ImageMetric};
        context.begin(1)?;
        let preparation_base = context.preparation_work_items();
        let mut held = 0_u64;
        let preparation = (|| {
            let mut progress = crate::context::WorkProgress::new(
                observer
                    .as_mut()
                    .map(|observer| &mut **observer as &mut dyn crate::MetricWorkObserver),
            );
            progress.context_poll(context)?;
            let (target, selected) =
                self.image_target_with_progress(source.crs(), target, context, &mut progress)?;
            let source_reference =
                self.operation_reference_with_progress(source.crs(), context, &mut progress)?;
            let target_reference =
                self.operation_reference_with_progress(target, context, &mut progress)?;
            let chain = if let Some(chain) = selected {
                chain
            } else {
                self.operation_between_with_progress(source.crs(), target, context, &mut progress)?
            };
            if chain.operations().first().expect("compiled chain").source() != source_reference
                || chain.operations().last().expect("compiled chain").target() != target_reference
            {
                return Err(CarrierResolutionFailure::ImageEndpoints {
                    reason: crate::ImageEndpointReason::Chain,
                    expected_source: source_reference,
                    actual_source: chain.operations().first().expect("compiled chain").source(),
                    expected_target: target_reference,
                    actual_target: chain.operations().last().expect("compiled chain").target(),
                }
                .retained(&mut progress));
            }
            let admitted_source = crate::operation::AdmittedImageSource::prepare(
                source.geometry(),
                context,
                &mut progress,
            )?;
            held = admitted_source.workspace_bytes();
            if source_reference.unit == CoordinateUnit::Degrees {
                let reference =
                    self.reference_with_progress(source.crs(), context, &mut progress)?;
                Self::validate_literal_coordinates(
                    source,
                    reference,
                    Some((context, &mut progress)),
                )?;
            }
            let target_metric = match target_reference.unit {
                CoordinateUnit::Degrees => {
                    Some(self.reference_with_progress(target, context, &mut progress)?)
                }
                CoordinateUnit::Metres => None,
            };
            let metric_bytes = target_metric.map_or(Ok(0), |reference| {
                (size_of::<GeographicReference>() as u64)
                    .checked_add(reference.ellipsoid().retained_limb_bytes())
                    .ok_or(GeoError::ArithmeticOverflow(
                        "image metric preparation storage",
                    ))
            })?;
            let bytes = (target.as_str().len() as u64)
                .checked_add(metric_bytes)
                .ok_or(GeoError::ArithmeticOverflow(
                    "image carrier preparation storage",
                ))?;
            context.admit_workspace(bytes)?;
            held = held.checked_add(bytes).ok_or(GeoError::ArithmeticOverflow(
                "image source and carrier preparation storage",
            ))?;
            progress.context_poll(context)?;
            let metric = if let Some(reference) = target_metric {
                progress.exact(
                    context,
                    crate::numerical::reference_copy_cost(reference)?,
                    || Ok(ImageMetric::Geographic(Box::new(reference.clone()))),
                )?
            } else {
                ImageMetric::CartesianMetres
            };
            let copy = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
                purrdf_xsd::integer::ExactOperation::Linear,
                (target.as_str().len() as u64)
                    .checked_mul(8)
                    .ok_or(GeoError::ArithmeticOverflow("image carrier copy work"))?,
                1,
            )
            .ok_or(GeoError::ArithmeticOverflow("image carrier copy work"))?;
            let target = progress.exact(context, copy, || Ok(target.clone()))?;
            context.retain_current_preparation()?;
            Ok((chain, admitted_source, target, metric))
        })();
        let (chain, admitted_source, target, metric) = match preparation {
            Ok(prepared) => prepared,
            Err(error) => {
                context.release_workspace(held)?;
                return Err(error);
            }
        };
        let preparation_work = context
            .preparation_work_items()
            .checked_sub(preparation_base)
            .ok_or(GeoError::ArithmeticOverflow(
                "image carrier preparation work",
            ))?;
        let result = if let Some(observer) = observer {
            let mut continuation = crate::MetricWorkContinuation::new(
                observer,
                context.work_items(),
                context.workspace_peak(),
            );
            chain.materialize_admitted_image(
                admitted_source,
                target,
                &metric,
                epoch,
                context,
                Some(&mut continuation),
            )
        } else {
            chain.materialize_admitted_image(admitted_source, target, &metric, epoch, context, None)
        };
        drop(metric);
        context.release_preparation_admission(preparation_work, held)?;
        result
    }

    /// Decode original source axes into a validated exact longitude/latitude.
    ///
    /// The source coordinate remains untouched, including its Z and M ordinates.
    ///
    /// # Errors
    ///
    /// Refuses an unregistered reference or an original out-of-range coordinate.
    pub fn lon_lat(&self, crs: &Crs, coordinate: &Coord) -> Result<LonLat, GeoError> {
        let (longitude, latitude) = match self.reference(crs)?.axes() {
            AxisOrder::LonLat => (coordinate.x(), coordinate.y()),
            AxisOrder::LatLon => (coordinate.y(), coordinate.x()),
        };
        LonLat::new(longitude.clone(), latitude.clone())
    }

    /// Validate every original carrier coordinate before numerical conversion.
    ///
    /// # Errors
    ///
    /// Refuses unregistered references and out-of-range original axes.
    pub fn validate_literal(&self, literal: &GeometryLiteral) -> Result<(), GeoError> {
        Self::validate_literal_coordinates(literal, self.reference(literal.crs())?, None)
    }

    /// Validate borrowed original axes under the worker's remaining exact-work
    /// and scratch admission. This continues the current invocation unchanged.
    /// # Errors
    /// Preserves original reference/range errors and fatal resource refusals.
    pub fn validate_literal_in_context(
        &self,
        literal: &GeometryLiteral,
        context: &mut crate::MetricContext,
    ) -> Result<(), GeoError> {
        self.validate_literal_observed(literal, context, None)
    }

    /// Validate the same original coordinates with bounded governor polling.
    /// # Errors
    /// Adds observer refusal to [`Self::validate_literal_in_context`].
    pub fn validate_literal_in_context_metered(
        &self,
        literal: &GeometryLiteral,
        context: &mut crate::MetricContext,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.validate_literal_observed(literal, context, Some(observer))
    }

    fn validate_literal_observed(
        &self,
        literal: &GeometryLiteral,
        context: &mut crate::MetricContext,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        let mut progress = crate::context::WorkProgress::integer(observer);
        progress.context_poll(context)?;
        let reference = self.reference_with_progress(literal.crs(), context, &mut progress)?;
        let workspace = crate::operation::AdmittedImageSource::prepare(
            literal.geometry(),
            context,
            &mut progress,
        )?
        .workspace_bytes();
        let result =
            Self::validate_literal_coordinates(literal, reference, Some((context, &mut progress)));
        let released = context.release_workspace(workspace);
        progress.settle_result(result, released)
    }

    pub(crate) fn validate_literal_coordinates(
        literal: &GeometryLiteral,
        reference: &GeographicReference,
        admission: Option<(
            &mut crate::MetricContext,
            &mut crate::context::WorkProgress<'_>,
        )>,
    ) -> Result<(), GeoError> {
        let mut resident = crate::context::WorkProgress::integer(None);
        let (mut context, progress) = match admission {
            Some((context, progress)) => (Some(context), progress),
            None => (None, &mut resident),
        };
        crate::geom::visit_coords_with_progress(
            literal.geometry(),
            progress,
            |coordinate, progress| {
                let (longitude, latitude) = match reference.axes() {
                    AxisOrder::LonLat => (coordinate.x(), coordinate.y()),
                    AxisOrder::LatLon => (coordinate.y(), coordinate.x()),
                };
                let mut admission = context
                    .as_deref_mut()
                    .map(|context| crate::numerical::ExactAdmission::new(context, progress));
                crate::numerical::exact_rational_counted(
                    admission.as_mut(),
                    purrdf_xsd::integer::ExactOperation::RationalCompare,
                    &[longitude, latitude],
                    4,
                    |values| LonLat::validate_with(longitude, latitude, values),
                )??;
                Ok(())
            },
        )
    }

    /// The shared query identity tuple for every evaluator and host lane.
    #[must_use]
    pub fn query_identity(&self) -> GeoQueryIdentity {
        self.query_identity_with_binding(self.binding_id())
    }

    fn query_identity_with_binding(&self, binding: GeoBindingId) -> GeoQueryIdentity {
        GeoQueryIdentity {
            distance_law: PointLaw::ShortestDistanceMicrometreV1.id(),
            inverse_law: PointLaw::InverseMetadataV1.id(),
            direct_law: PointLaw::DirectDegree15V1.id(),
            geometry_distance_law: crate::ellipsoidal::GeometryMetricLaw::Distance.id(),
            length_law: crate::ellipsoidal::GeometryMetricLaw::Length.id(),
            perimeter_law: crate::ellipsoidal::GeometryMetricLaw::Perimeter.id(),
            area_law: crate::ellipsoidal::GeometryMetricLaw::Area.id(),
            geodesic_area_integral_law: crate::ellipsoidal::GeometryMetricLaw::GeodesicAreaIntegral
                .id(),
            topology_law: crate::atlas::topology_law_id(),
            image_materialization_law: crate::operation::GeometryImage::output_law_id(),
            point_buffer_materialization_law: crate::buffer::BufferMaterialization::output_law_id(),
            buffer_materialization_law: crate::buffer::BufferMaterialization::region_output_law_id(
            ),
            global_point_buffer_materialization_law:
                crate::buffer::BufferMaterialization::global_output_law_id(),
            global_buffer_materialization_law:
                crate::buffer::BufferMaterialization::global_region_output_law_id(),
            binding,
            policy: self.policy().id(),
        }
    }

    /// Identity of carrier bindings; admission limits do not enter this preimage.
    #[must_use]
    pub fn binding_id(&self) -> GeoBindingId {
        *self.binding.get_or_init(|| self.compute_binding_id())
    }

    fn compute_binding_id(&self) -> GeoBindingId {
        self.compute_binding_with(
            |reference| Ok::<_, core::convert::Infallible>(reference.id()),
            |value, hash| {
                frame_le_into(hash, value.numerator().to_string().as_bytes());
                frame_le_into(hash, value.denominator().to_string().as_bytes());
                Ok(())
            },
        )
        .expect("resident profile identity rendering")
    }

    fn compute_binding_with<E>(
        &self,
        mut identity: impl FnMut(&GeographicReference) -> Result<GeoBindingId, E>,
        mut rational: impl FnMut(&Rat, &mut blake3::Hasher) -> Result<(), E>,
    ) -> Result<GeoBindingId, E> {
        let mut hash = blake3::Hasher::new();
        hash.update(PROFILE_BINDING_DOMAIN.as_bytes());
        frame_le_into(&mut hash, ogc::CRS84.as_bytes());
        frame_le_into(&mut hash, identity(&CRS84_REFERENCE)?.digest().as_bytes());
        for entry in &self.references {
            frame_le_into(&mut hash, entry.crs.as_str().as_bytes());
            frame_le_into(&mut hash, identity(&entry.reference)?.digest().as_bytes());
        }
        for operation in &self.operations {
            frame_le_into(&mut hash, b"explicit-operation-chain");
            frame_le_into(&mut hash, operation.name.as_str().as_bytes());
            frame_le_into(&mut hash, operation.source.as_str().as_bytes());
            frame_le_into(&mut hash, operation.target.as_str().as_bytes());
            frame_le_into(&mut hash, operation.chain.id().as_bytes());
        }
        frame_le_into(&mut hash, b"linear-output-units");
        frame_le_into(&mut hash, ogc::uom::METRE.as_bytes());
        frame_le_into(&mut hash, b"1");
        frame_le_into(&mut hash, b"1");
        for unit in &self.units {
            frame_le_into(&mut hash, unit.unit.as_str().as_bytes());
            rational(&unit.metres_per_unit, &mut hash)?;
        }
        Ok(GeoBindingId::from_digest(Digest32::new(
            *hash.finalize().as_bytes(),
        )))
    }

    fn binding_id_using(
        &self,
        values: &crate::numerical::ExactValues<'_>,
    ) -> Result<GeoBindingId, GeoError> {
        if let Some(identity) = self.binding.get() {
            return Ok(*identity);
        }
        let identity = self.compute_binding_with(
            |reference| -> Result<_, GeoError> {
                if let Some(identity) = reference.cached_identity() {
                    return Ok(identity);
                }
                let identity = crate::numerical::reference_identity_using(reference, values)?;
                reference.cache_identity(identity);
                Ok(identity)
            },
            |value, hash| {
                for integer in [value.numerator(), value.denominator()] {
                    let digits = values
                        .decimal_digits(&integer)
                        .map_err(GeoError::NumericalScratch)?;
                    crate::numerical::frame_decimal_digits(hash, &digits)?;
                }
                Ok(())
            },
        )?;
        let _ = self.binding.set(identity);
        Ok(identity)
    }

    /// Share an immutable profile's original tables, strings, model and numeric
    /// owners. A profile clone has no new heap payload; registration publishes a
    /// new admitted table while existing clones keep their original table.
    ///
    /// # Errors
    /// Refuses cumulative configuration work/storage before any copy allocation.
    pub fn clone_in_budget(&self, budget: &mut crate::PreparationBudget) -> Result<Self, GeoError> {
        self.retain_configuration_in_budget(budget)?;
        budget.retain(1, 0)?;
        Ok(self.clone())
    }

    // Configuration owns its reachable immutable graph independently of later
    // worker scratch. Admit bounded metadata walks before scanning model nodes.
    /// Retain the complete original immutable profile graph before an admitted
    /// configuration/export operation visits or copies its owners.
    ///
    /// # Errors
    /// Refuses checked source-work/storage exhaustion before output construction.
    pub fn retain_configuration_in_budget(
        &self,
        budget: &mut crate::PreparationBudget,
    ) -> Result<(), GeoError> {
        let mut bytes = self
            .references
            .admitted_bytes()
            .checked_add(self.operations.admitted_bytes())
            .and_then(|bytes| bytes.checked_add(self.units.admitted_bytes()))
            .and_then(|bytes| u64::try_from(bytes).ok())
            .and_then(|bytes| bytes.checked_add(size_of::<Self>() as u64))
            .ok_or(GeoError::ArithmeticOverflow("immutable profile storage"))?;
        for reference in &self.references {
            budget.retain(1, 0)?;
            bytes = bytes
                .checked_add(reference.crs.retained_text_bytes() as u64)
                .and_then(|bytes| {
                    bytes.checked_add(reference.reference.ellipsoid().retained_limb_bytes())
                })
                .ok_or(GeoError::ArithmeticOverflow("immutable reference storage"))?;
        }
        for operation in &self.operations {
            budget.retain(operation.chain.storage_walk_work()?, 0)?;
            bytes = bytes
                .checked_add(operation.chain.retained_workspace_bytes()?)
                .and_then(|bytes| bytes.checked_add(operation.name.retained_text_bytes() as u64))
                .and_then(|bytes| bytes.checked_add(operation.source.retained_text_bytes() as u64))
                .and_then(|bytes| bytes.checked_add(operation.target.retained_text_bytes() as u64))
                .ok_or(GeoError::ArithmeticOverflow("immutable operation storage"))?;
        }
        for unit in &self.units {
            budget.retain(1, 0)?;
            bytes = bytes
                .checked_add(unit.unit.retained_text_bytes() as u64)
                .and_then(|bytes| bytes.checked_add(unit.metres_per_unit.allocated_bytes() as u64))
                .ok_or(GeoError::ArithmeticOverflow("immutable unit storage"))?;
        }
        budget.retain(0, bytes)
    }

    fn registration_in_budget<R, T>(
        &mut self,
        candidate_bytes: u64,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        table_length: usize,
        budget: &mut crate::PreparationBudget,
        evaluate: impl FnOnce(
            &mut Self,
            &crate::numerical::ExactValues<'_>,
            &mut crate::context::WorkProgress<'_>,
        ) -> Result<R, GeoError>,
        mut observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<R, GeoError> {
        let required = table_length.checked_add(1).ok_or(GeoError::Storage(
            purrdf_lex::allocation::StorageError::SizeOverflow,
        ))?;
        let replacement = {
            let progress = crate::context::WorkProgress::integer(
                observer
                    .as_mut()
                    .map(|observer| &mut **observer as &mut dyn crate::MetricWorkObserver),
            );
            crate::context::RetainedBuilder::<T>::default()
                .replacement_bytes(required, &progress)?
                .ok_or(GeoError::ArithmeticOverflow(
                    "native registration table layout",
                ))?
        };
        let replacement = u64::try_from(replacement)
            .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?;
        budget.with_workspace(candidate_bytes, |phase| {
            let before = phase.workspace_bytes();
            self.retain_configuration_in_budget(phase)?;
            let graph_bytes = phase.workspace_bytes() - before;
            let binding = self.binding_cost(|| phase.retain(1, 0))?;
            // Table comparisons, complete equal-name model comparisons and
            // insertion shifts visit no more than the live graph's bytes.
            let scan_bits = graph_bytes
                .checked_add(candidate_bytes)
                .and_then(|bytes| bytes.checked_mul(8))
                .ok_or(GeoError::ArithmeticOverflow("registration scan width"))?;
            let arithmetic = cost
                .followed_by(binding)
                .ok_or(GeoError::ArithmeticOverflow("registration work"))?;
            let mut cost = arithmetic
                .followed_by(
                    purrdf_xsd::integer::ExactArithmeticCost::for_operation(
                        purrdf_xsd::integer::ExactOperation::Linear,
                        scan_bits,
                        8,
                    )
                    .ok_or(GeoError::ArithmeticOverflow("registration scan work"))?,
                )
                .ok_or(GeoError::ArithmeticOverflow("registration work"))?;
            // The existing input tables remain admitted throughout the new
            // buffer/control birth. The shared builder supplies its exact
            // replacement layout, including the original observer's owner.
            cost.workspace_bytes = cost
                .workspace_bytes
                .checked_add(replacement)
                .ok_or(GeoError::ArithmeticOverflow("registration table growth"))?;
            // Scanning the aggregate table bytes creates no integer of that
            // width. Only the original numerical fields and their renderers
            // determine reusable arithmetic destinations.
            phase.exact_values_with_bits_observed(
                cost,
                arithmetic.output_bits,
                |values, progress| {
                    let previous = self.clone();
                    let result = evaluate(self, values, progress)
                        .and_then(|result| self.binding_id_using(values).map(|_| result));
                    if result.is_err() {
                        *self = previous;
                    }
                    result
                },
                observer,
            )
        })
    }

    /// Compile the immutable query tuple under an explicit configuration budget.
    /// Completed mathematical/binding IDs are independent of this admission.
    ///
    /// # Errors
    /// Refuses before unadmitted integer rendering or binding preimage hashing.
    pub fn query_identity_in_budget(
        &self,
        budget: &mut crate::PreparationBudget,
    ) -> Result<GeoQueryIdentity, GeoError> {
        self.query_identity_observed(budget, None)
    }

    /// Compile the same immutable query identity under native physical admission.
    ///
    /// # Errors
    /// Preserves original storage, integer, work and cancellation refusals.
    pub fn query_identity_in_budget_metered(
        &self,
        budget: &mut crate::PreparationBudget,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<GeoQueryIdentity, GeoError> {
        self.query_identity_observed(budget, Some(observer))
    }

    fn query_identity_observed(
        &self,
        budget: &mut crate::PreparationBudget,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<GeoQueryIdentity, GeoError> {
        self.retain_configuration_in_budget(budget)?;
        let cost = self.binding_cost(|| budget.retain(1, 0))?;
        let binding = budget.exact_values_observed(
            cost,
            |values, _| self.binding_id_using(values),
            observer,
        )?;
        budget.retain(0, size_of::<GeoQueryIdentity>() as u64)?;
        Ok(self.query_identity_with_binding(binding))
    }

    pub(crate) fn binding_id_in_context(
        &self,
        context: &mut crate::MetricContext,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<GeoBindingId, GeoError> {
        let cost = self.binding_cost(|| {
            context.charge_work(1)?;
            progress.context_poll(context)
        })?;
        crate::numerical::ExactAdmission::new(context, progress)
            .expression_cost(cost, |values| self.binding_id_using(values))
    }

    // The pure, prepared-index and host constructors admit one actual binding
    // body; this worker bounds cold reference IDs as well as unit renderings.
    fn binding_cost(
        &self,
        mut poll: impl FnMut() -> Result<(), GeoError>,
    ) -> Result<purrdf_xsd::integer::ExactArithmeticCost, GeoError> {
        use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
        let mut text_bytes = 512_u64;
        let mut cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, 1, 1)
            .ok_or(GeoError::ArithmeticOverflow("carrier binding work"))?;
        for reference in std::iter::once(&*CRS84_REFERENCE)
            .chain(self.references.iter().map(|entry| &entry.reference))
        {
            poll()?;
            for value in [
                reference.ellipsoid().semimajor(),
                reference.ellipsoid().inverse_flattening(),
            ] {
                let (render, bytes) = crate::numerical::rational_field_cost(value)?;
                text_bytes = text_bytes
                    .checked_add(bytes)
                    .ok_or(GeoError::ArithmeticOverflow("reference binding text"))?;
                cost = cost
                    .followed_by(render)
                    .ok_or(GeoError::ArithmeticOverflow("carrier binding work"))?;
            }
        }
        for entry in &self.references {
            poll()?;
            text_bytes = text_bytes
                .checked_add(entry.crs.as_str().len() as u64)
                .and_then(|bytes| bytes.checked_add(128))
                .ok_or(GeoError::ArithmeticOverflow("reference binding bytes"))?;
        }
        for entry in &self.operations {
            poll()?;
            for text in [&entry.name, &entry.source, &entry.target] {
                text_bytes = text_bytes
                    .checked_add(text.as_str().len() as u64)
                    .and_then(|bytes| bytes.checked_add(128))
                    .ok_or(GeoError::ArithmeticOverflow("operation binding bytes"))?;
            }
        }
        for entry in &self.units {
            poll()?;
            text_bytes = text_bytes
                .checked_add(entry.unit.as_str().len() as u64)
                .and_then(|bytes| bytes.checked_add(128))
                .ok_or(GeoError::ArithmeticOverflow("unit binding bytes"))?;
            let (render, bytes) = crate::numerical::rational_field_cost(&entry.metres_per_unit)?;
            text_bytes = text_bytes
                .checked_add(bytes)
                .ok_or(GeoError::ArithmeticOverflow("unit binding text"))?;
            cost = cost
                .followed_by(render)
                .ok_or(GeoError::ArithmeticOverflow("carrier binding work"))?;
        }
        let arithmetic_bits = cost.output_bits;
        cost = cost
            .followed_by(
                ExactArithmeticCost::for_operation(
                    ExactOperation::Linear,
                    text_bytes
                        .checked_mul(8)
                        .ok_or(GeoError::ArithmeticOverflow("carrier binding bits"))?,
                    8,
                )
                .ok_or(GeoError::ArithmeticOverflow("carrier binding hashing"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("carrier binding work"))?;
        // The preimage is streamed into the hasher. Its total size prices the
        // encoding work and storage, never a single numerical destination.
        cost.output_bits = arithmetic_bits;
        cost.workspace_bytes = cost
            .workspace_bytes
            .checked_add(text_bytes)
            .ok_or(GeoError::ArithmeticOverflow("carrier binding scratch"))?;
        Ok(cost)
    }
}

/// Exact dimensional conversion for physical linear and areal output metrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinearUnitBinding {
    unit: Crs,
    metres_per_unit: Rat,
}

/// One admission home for the profile's original borrowed text lookups. The
/// count is checked before reading metadata; its linear visit bound covers
/// both binary lookup and complete carrier scans, plus original error copies.
fn carrier_lookup_cost<'a>(
    queries: &[&Crs],
    candidates: impl Iterator<Item = &'a Crs>,
    count: usize,
    context: &mut crate::MetricContext,
    progress: &mut crate::context::WorkProgress<'_>,
) -> Result<purrdf_xsd::integer::ExactArithmeticCost, GeoError> {
    let count =
        u64::try_from(count).map_err(|_| GeoError::ArithmeticOverflow("carrier lookup count"))?;
    context.charge_work(
        count
            .checked_add(queries.len() as u64)
            .and_then(|items| items.checked_add(1))
            .ok_or(GeoError::ArithmeticOverflow("carrier lookup metadata"))?,
    )?;
    progress.context_poll(context)?;
    let mut text = 0_u64;
    let mut owned = 0_u64;
    let comparisons = count
        .checked_add(2)
        .ok_or(GeoError::ArithmeticOverflow("carrier lookup visits"))?;
    for query in queries {
        text = text
            .checked_add(
                (query.as_str().len() as u64)
                    .checked_mul(comparisons)
                    .ok_or(GeoError::ArithmeticOverflow("carrier lookup text"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("carrier query text"))?;
        owned = owned
            .checked_add(query.retained_text_bytes() as u64)
            .ok_or(GeoError::ArithmeticOverflow("carrier query ownership"))?;
    }
    for (index, candidate) in candidates.enumerate() {
        if index.is_multiple_of(256) {
            progress.context_poll(context)?;
        }
        text = text
            .checked_add(candidate.as_str().len() as u64)
            .ok_or(GeoError::ArithmeticOverflow("carrier candidate text"))?;
    }
    let (work_items, workspace_bytes) =
        crate::carrier::InputAdmission::text_allowance(text, owned)?;
    Ok(purrdf_xsd::integer::ExactArithmeticCost {
        work_items,
        workspace_bytes,
        output_bits: 0,
    })
}

fn geographic_operation_reference(
    reference: &GeographicReference,
    identity: GeoBindingId,
) -> crate::operation::OperationReference {
    crate::operation::OperationReference {
        realization: identity.digest(),
        unit: crate::operation::CoordinateUnit::Degrees,
        swapped_axes: reference.axes() == AxisOrder::LatLon,
    }
}

impl LinearUnitBinding {
    /// The explicitly declared exact output-unit IRI.
    #[must_use]
    pub const fn unit(&self) -> &Crs {
        &self.unit
    }

    /// The positive exact number of metres in one linear unit.
    #[must_use]
    pub const fn metres_per_unit(&self) -> &Rat {
        &self.metres_per_unit
    }
}

#[allow(
    clippy::result_large_err,
    reason = "Original endpoint identities remain on the stack until the caller admits their owned diagnostic; a box here would create an unpriced allocation"
)]
fn validate_endpoint_resolution<'a>(
    carrier: &'a Crs,
    reference: &GeographicReference,
    endpoint: crate::operation::OperationReference,
) -> Result<(), CarrierResolutionFailure<'a>> {
    let expected = geographic_operation_reference(reference, reference.id());
    if endpoint != expected {
        return Err(CarrierResolutionFailure::InvalidEndpoint {
            carrier,
            expected,
            actual: endpoint,
        });
    }
    Ok(())
}

/// Immutable named coordinate chain and its exact source/target carriers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationBinding {
    name: Crs,
    source: Crs,
    target: Crs,
    chain: OperationChain,
}

impl OperationBinding {
    /// The caller-supplied exact dispatch IRI.
    #[must_use]
    pub const fn name(&self) -> &Crs {
        &self.name
    }
    /// The exact source carrier IRI.
    #[must_use]
    pub const fn source(&self) -> &Crs {
        &self.source
    }
    /// The exact target carrier IRI.
    #[must_use]
    pub const fn target(&self) -> &Crs {
        &self.target
    }
    /// The immutable compiled mathematical operation chain.
    #[must_use]
    pub const fn chain(&self) -> &OperationChain {
        &self.chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::{
        CoordinateOperation, CoordinateUnit, OperationModel, OperationReference,
    };
    use crate::{MetricContext, MetricWorkObserver, Rat};

    #[test]
    fn native_reference_refusals_keep_original_declarations_without_warming_identity() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let cold = GeographicReference::cgcs2000();
        assert!(cold.cached_identity().is_none());
        assert_eq!(
            Capture::read(&cold).child("axes").code,
            "axis-order-longitude-latitude"
        );
        assert!(cold.cached_identity().is_none());

        for (name, actual, expected, code) in [
            (
                ogc::CRS84,
                cold.clone(),
                GeographicReference::wgs84(),
                "reference-registration-crs84",
            ),
            (
                ogc::EPSG4326,
                GeographicReference::wgs84(),
                GeographicReference::wgs84().with_axes(AxisOrder::LatLon),
                "reference-registration-epsg4326",
            ),
            (
                "https://example.org/reference",
                cold,
                GeographicReference::wgs84(),
                "reference-registration-conflicting",
            ),
        ] {
            let mut profile = GeoProfile::standard();
            let carrier = Crs::new(name).unwrap();
            if code == "reference-registration-conflicting" {
                profile
                    .register_reference(carrier.clone(), expected.clone())
                    .unwrap();
                profile
                    .register_reference(carrier.clone(), expected.clone())
                    .unwrap();
            }
            let error = profile
                .register_reference(carrier, actual.clone())
                .unwrap_err();
            let clone = error.clone();
            drop((profile, error));
            let fields = Capture::read(clone.original()).child("cause");
            assert_eq!(fields.code, "carrier-reference-registration");
            assert_eq!(fields.child("reason").code, code);
            assert!(matches!(fields.field("reference"), ErrorValue::Iri(value) if value == name));
            assert!(matches!(
                fields.child("actual").field("ellipsoid"),
                ErrorValue::Record(_)
            ));
            let GeoError::BindingConfiguration(cause) = clone.original() else {
                panic!("native reference refusal")
            };
            let crate::BindingConfigError::ReferenceRegistration {
                actual: original_actual,
                expected: original_expected,
                ..
            } = &**cause
            else {
                panic!("original reference declarations")
            };
            assert_eq!(original_actual, &actual);
            assert_eq!(original_expected, &expected);
        }
    }

    #[test]
    fn native_unit_refusals_keep_exact_factors_and_registration_priority() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        for (name, factor, expected, code) in [
            (ogc::uom::METRE, -2, None, "unit-factor-nonpositive"),
            (ogc::uom::METRE, 2, Some(1), "unit-factor-official-metre"),
            (
                "https://example.org/unit",
                4,
                Some(3),
                "unit-factor-conflicting-registration",
            ),
        ] {
            let mut profile = GeoProfile::standard();
            let unit = Crs::new(name).unwrap();
            if expected == Some(3) {
                profile
                    .register_linear_unit(unit.clone(), Rat::from_i64(3))
                    .unwrap();
                // An identical re-registration remains accepted.
                profile
                    .register_linear_unit(unit.clone(), Rat::from_i64(3))
                    .unwrap();
            }
            let error = profile
                .register_linear_unit(unit, Rat::from_i64(factor))
                .unwrap_err();
            let clone = error.clone();
            drop((profile, error));
            let fields = Capture::read(clone.original()).child("cause");
            assert_eq!(fields.code, "carrier-unit-factor");
            assert_eq!(fields.child("reason").code, code);
            assert!(matches!(fields.field("unit"), ErrorValue::Iri(value) if value == name));
            assert!(matches!(fields.field("actual"), ErrorValue::Record(_)));
            let GeoError::BindingConfiguration(cause) = clone.original() else {
                panic!("native unit refusal")
            };
            let crate::BindingConfigError::UnitFactor {
                actual,
                expected: original,
                ..
            } = &**cause
            else {
                panic!("original exact unit factors")
            };
            assert_eq!(&**actual, &Rat::from_i64(factor));
            assert_eq!(
                original.as_ref().map(|value| &**value),
                expected.map(Rat::from_i64).as_ref()
            );
            if expected.is_none() {
                assert!(matches!(fields.field("expected"), ErrorValue::Absent));
            }
        }
    }

    #[test]
    fn unresolved_output_unit_keeps_its_original_identifier_and_domain_class() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let profile = GeoProfile::standard();
        let unit = Crs::new("https://example.org/unit/未知").unwrap();
        let error = profile.metres_per_unit_value(&unit).unwrap_err();
        drop(unit);
        drop(profile);
        let last = error.clone();
        drop(error);
        let fields = Capture::read(&last).child("cause");
        assert_eq!(fields.code, "vocabulary-unregistered-output-unit");
        assert!(matches!(
            fields.field("unit"),
            ErrorValue::Iri("https://example.org/unit/未知")
        ));
        assert!(last.is_expression_error());
        assert_eq!(
            last.detail(),
            "unregistered linear output unit <https://example.org/unit/未知>"
        );
        assert_eq!(
            GeoProfile::standard()
                .metres_per_unit(&Crs::new(ogc::uom::METRE).unwrap())
                .unwrap(),
            Rat::one()
        );
    }

    #[test]
    fn native_metric_selection_refusals_keep_the_original_carrier_pair() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let empty = GeoProfile::standard().metric_reference(&[]).unwrap_err();
        assert_eq!(
            Capture::read(&empty).child("cause").code,
            "carrier-empty-metric-sources"
        );
        assert_eq!(empty.detail(), "metric reference requires a source carrier");

        let first = Crs::new("https://example.org/first-surface").unwrap();
        let second = Crs::new("https://example.org/second-surface").unwrap();
        let resident = CarrierResolutionFailure::AmbiguousMetricSurface {
            first: &first,
            second: &second,
        }
        .resident();
        let mut progress = crate::context::WorkProgress::integer(None);
        let metered = CarrierResolutionFailure::AmbiguousMetricSurface {
            first: &first,
            second: &second,
        }
        .retained(&mut progress);
        drop((first, second));
        for error in [resident, metered] {
            let clone = error.clone();
            drop(error);
            let fields = Capture::read(clone.original()).child("cause");
            assert_eq!(fields.code, "carrier-ambiguous-metric-surface");
            assert!(matches!(
                fields.field("first"),
                ErrorValue::Iri("https://example.org/first-surface")
            ));
            assert!(matches!(
                fields.field("second"),
                ErrorValue::Iri("https://example.org/second-surface")
            ));
        }
    }

    #[test]
    fn native_operation_lookup_keeps_the_original_identifier_in_both_paths() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let profile = GeoProfile::standard();
        let name = Crs::new("https://example.org/original-operation").unwrap();
        let resident = profile.operation(&name).unwrap_err();
        let mut context = MetricContext::wgs84().unwrap();
        let mut progress = crate::context::WorkProgress::integer(None);
        let metered = profile
            .operation_with_progress(&name, &mut context, &mut progress)
            .unwrap_err();
        drop((name, profile));
        assert_eq!(resident.to_string(), metered.to_string());
        for error in [resident, metered] {
            let cloned = error.clone();
            drop(error);
            let fields = Capture::read(cloned.original()).child("cause");
            assert_eq!(fields.code, "carrier-unknown-operation");
            assert!(matches!(
                fields.field("operation"),
                ErrorValue::Iri("https://example.org/original-operation")
            ));
        }
    }

    #[test]
    fn refused_second_native_field_refunds_the_first_field_before_return() {
        use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};

        let source = Crs::new("https://example.org/source").unwrap();
        let target = Crs::new("https://example.org/target").unwrap();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident as &mut dyn Admission);
        // Exercise accounting overflow without allocating the huge baseline.
        // The first real string fits; admitting the second cannot fit usize.
        let baseline = usize::MAX - source.as_str().len();
        memory.add_bytes(baseline).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result =
            CarrierResolutionFailure::MissingOperation(&source, &target).with_memory(&mut memory);
        let allocations = window.close();
        assert!(matches!(result, Err(StorageError::SizeOverflow)));
        assert_eq!(allocations.allocations, 1);
        assert_eq!(allocations.requested_bytes, source.as_str().len() as u64);
        assert_eq!(memory.admitted_bytes(), baseline);
    }

    #[test]
    fn native_endpoint_refusal_keeps_required_and_actual_reference_fields() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let carrier = Crs::new("https://example.org/original-carrier").unwrap();
        let reference = GeographicReference::wgs84();
        let actual = OperationReference {
            realization: Digest32::new([7; 32]),
            unit: CoordinateUnit::Metres,
            swapped_axes: true,
        };
        let error = validate_endpoint_resolution(&carrier, &reference, actual)
            .unwrap_err()
            .resident();
        drop((carrier, reference));
        let fields = Capture::read(&error).child("cause");
        assert_eq!(fields.code, "carrier-invalid-endpoint");
        assert!(matches!(
            fields.field("carrier"),
            ErrorValue::Iri("https://example.org/original-carrier")
        ));
        assert_eq!(
            fields.child("expected").child("unit").code,
            "operation-unit-degrees"
        );
        let actual = fields.child("actual");
        assert_eq!(actual.child("unit").code, "operation-unit-metres");
        assert!(matches!(
            actual.field("swapped_axes"),
            ErrorValue::Boolean(true)
        ));
        assert!(
            matches!(actual.child("realization").field("bytes"), ErrorValue::Bytes(bytes) if bytes == [7; 32])
        );
    }

    #[test]
    fn native_binding_conflicts_keep_original_chain_ids_and_endpoint_axes() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let (mut profile, target) = projected_profile();
        let source = Crs::new(ogc::CRS84).unwrap();
        let first = profile.operation_between(&source, &target).unwrap().id();
        let mut endpoint = profile.operation_reference(&target).unwrap();
        endpoint.swapped_axes = true;
        let operation = CoordinateOperation::compile(
            profile.operation_reference(&source).unwrap(),
            endpoint,
            OperationModel::WebMercator {
                radius: Rat::from_i64(6_371_000),
            },
        )
        .unwrap();
        let chain = OperationChain::compile(vec![operation]).unwrap();
        let second = chain.id();
        let registration = profile
            .register_operation(
                Crs::new("http://example.org/project").unwrap(),
                source.clone(),
                target.clone(),
                chain.clone(),
            )
            .unwrap_err();
        profile
            .register_operation(
                Crs::new("https://example.org/second-operation").unwrap(),
                source.clone(),
                target.clone(),
                chain,
            )
            .unwrap();
        let chains = profile.operation_between(&source, &target).unwrap_err();
        let endpoints = profile.operation_reference(&target).unwrap_err();
        drop((profile, source, target));
        let fields = Capture::read(registration.original()).child("cause");
        assert_eq!(fields.code, "carrier-conflicting-operation");
        assert!(matches!(
            fields.field("operation"),
            ErrorValue::Iri("http://example.org/project")
        ));
        assert!(matches!(
            fields.field("first_source"),
            ErrorValue::Iri(ogc::CRS84)
        ));
        assert!(matches!(
            fields.field("second_target"),
            ErrorValue::Iri("http://example.org/projected")
        ));
        assert!(
            matches!(fields.child("first_chain").field("bytes"), ErrorValue::Bytes(bytes) if bytes == first.as_bytes())
        );
        assert!(
            matches!(fields.child("second_chain").field("bytes"), ErrorValue::Bytes(bytes) if bytes == second.as_bytes())
        );
        let fields = Capture::read(&chains).child("cause");
        assert_eq!(fields.code, "carrier-multiple-operations");
        assert!(
            matches!(fields.child("first").field("bytes"), ErrorValue::Bytes(bytes) if bytes == first.as_bytes())
        );
        assert!(
            matches!(fields.child("second").field("bytes"), ErrorValue::Bytes(bytes) if bytes == second.as_bytes())
        );
        let fields = Capture::read(&endpoints).child("cause");
        assert_eq!(fields.code, "carrier-conflicting-endpoint");
        assert!(matches!(
            fields.child("first").field("swapped_axes"),
            ErrorValue::Boolean(false)
        ));
        assert!(matches!(
            fields.child("second").field("swapped_axes"),
            ErrorValue::Boolean(true)
        ));
    }

    fn projected_profile() -> (GeoProfile, Crs) {
        let mut profile = GeoProfile::standard();
        let source = Crs::new(ogc::CRS84).unwrap();
        let target = Crs::new("http://example.org/projected").unwrap();
        let operation = CoordinateOperation::compile(
            profile.operation_reference(&source).unwrap(),
            OperationReference {
                realization: Digest32::new([41; 32]),
                unit: CoordinateUnit::Metres,
                swapped_axes: false,
            },
            OperationModel::WebMercator {
                radius: Rat::from_i64(6_378_137),
            },
        )
        .unwrap();
        profile
            .register_operation(
                Crs::new("http://example.org/project").unwrap(),
                source,
                target.clone(),
                OperationChain::compile(vec![operation]).unwrap(),
            )
            .unwrap();
        (profile, target)
    }

    #[derive(Default)]
    struct ImageWork {
        work: u64,
        peak: u64,
    }
    impl MetricWorkObserver for ImageWork {
        fn charge_chunk(&mut self, work: u64, workspace: u64) -> Result<(), GeoError> {
            self.work += work;
            self.peak += workspace;
            Ok(())
        }
    }

    #[test]
    fn image_adapter_admits_complete_source_before_validation_and_preserves_metered_receipts() {
        let (profile, target) = projected_profile();
        let source = crate::wkt::parse(
            "LINESTRING(1 1,1.000001 1.000001)",
            &Crs::new(ogc::CRS84).unwrap(),
        )
        .unwrap();
        let mut plain = MetricContext::wgs84().unwrap();
        let expected = profile
            .transform_literal(&source, &target, None, &mut plain)
            .unwrap();
        let mut metered = MetricContext::wgs84().unwrap();
        let preparation = metered.preparation_work_items();
        let mut observer = ImageWork::default();
        let result = profile
            .transform_literal_metered(&source, &target, None, &mut metered, &mut observer)
            .unwrap();
        assert_eq!(result, expected);
        assert_eq!(observer.work, metered.work_items());
        assert_eq!(observer.peak, metered.workspace_peak());
        assert_eq!(metered.preparation_work_items(), preparation);
        // Only actual reusable numerical storage survives the adapter scope.
        metered.begin(1).unwrap();
    }

    #[test]
    fn image_adapter_refuses_original_large_source_before_range_arithmetic_or_copy() {
        let (profile, target) = projected_profile();
        let tiny = Rat::new(crate::Int::one(), crate::Int::one().shl(16_384)).unwrap();
        let source = GeometryLiteral::new(
            Crs::new(ogc::CRS84).unwrap(),
            crate::Geometry::new(
                crate::CoordDim::Xy,
                crate::GeometryBody::Point(Some(Coord::xy(tiny, Rat::zero()))),
            )
            .unwrap(),
        );
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 64,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        assert!(matches!(
            profile.transform_literal(&source, &target, None, &mut context),
            Err(GeoError::WorkExhausted { limit: 64 })
        ));
        assert_eq!(context.retained_workspace_bytes(), 0);
        assert_eq!(context.preparation_work_items(), 0);
        context.begin(1).unwrap();
    }

    #[test]
    fn epsg_axes_require_explicit_wgs84_registration() {
        let epsg = Crs::new(ogc::EPSG4326).expect("official IRI");
        let mut profile = GeoProfile::standard();
        assert!(matches!(
            profile.reference(&epsg),
            Err(GeoError::UnregisteredCrs(_))
        ));
        assert!(
            profile
                .register_reference(epsg.clone(), GeographicReference::wgs84())
                .is_err()
        );
        profile
            .register_reference(
                epsg.clone(),
                GeographicReference::wgs84().with_axes(AxisOrder::LatLon),
            )
            .expect("explicit axes");
        let coordinate = Coord::xy(Rat::from_i64(49), Rat::from_i64(-123));
        assert_eq!(
            profile.lon_lat(&epsg, &coordinate).expect("swapped axes"),
            LonLat::new(Rat::from_i64(-123), Rat::from_i64(49)).expect("valid point")
        );
        assert_eq!(*coordinate.x(), Rat::from_i64(49));
    }

    #[test]
    fn protected_standard_and_profile_id_exclude_admission() {
        let crs = Crs::new(ogc::CRS84).expect("official IRI");
        let mut profile = GeoProfile::standard();
        let id = profile.binding_id();
        assert!(
            profile
                .register_reference(crs.clone(), GeographicReference::cgcs2000())
                .is_err()
        );
        profile
            .register_reference(crs, GeographicReference::wgs84())
            .expect("identical binding");
        assert_eq!(profile.binding_id(), id);
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_work_items += 1;
        let changed = profile.with_limits(limits).expect("valid raised limits");
        assert_eq!(changed.binding_id(), id);
        let standard = GeoProfile::STANDARD;
        assert_ne!(changed.policy().id(), standard.policy().id());
    }

    #[test]
    fn configuration_copies_admit_original_owners_before_allocating() {
        use crate::{Int, PreparationBudget};
        let generous = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 100_000_000,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut profile = GeoProfile::standard();
        profile
            .register_linear_unit_in_budget(
                Crs::new("http://example.org/large-copy-unit").unwrap(),
                Rat::from_int(Int::one().shl(4_096)),
                &mut PreparationBudget::new(generous),
            )
            .unwrap();
        for limits in [
            ExecutionLimits {
                max_work_items: 1,
                ..ExecutionLimits::GEOMETRY
            },
            ExecutionLimits {
                max_workspace_bytes: 1,
                ..ExecutionLimits::GEOMETRY
            },
        ] {
            let mut budget = PreparationBudget::new(ExecutionPolicy::new(limits).unwrap());
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = profile.clone_in_budget(&mut budget);
            let allocations = window.close();
            assert!(matches!(
                result,
                Err(GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. })
            ));
            assert_eq!(allocations.allocations, 0);
            assert_eq!(allocations.requested_bytes, 0);
        }
        let mut budget = PreparationBudget::new(generous);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let copy = profile.clone_in_budget(&mut budget).unwrap();
        let allocations = window.close();
        assert_eq!(copy, profile);
        // The immutable original tables, lexical and numeric owners are
        // shared. Admission still covers the reachable source graph.
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);
        assert!(budget.workspace_bytes() >= profile.units.admitted_bytes() as u64);
        assert!(u64::try_from(allocations.peak_working_bytes).unwrap() <= budget.workspace_peak());
    }

    #[test]
    fn admitted_binding_cache_preserves_original_sources_and_policy_separation() {
        use crate::{Int, PreparationBudget};
        let generous = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 100_000_000,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut profile = GeoProfile::standard();
        let initial = profile.binding_id();
        let unit = Crs::new("http://example.org/cached-unit").unwrap();
        let factor = Rat::from_int(Int::one().shl(4_096));
        for limits in [
            ExecutionLimits {
                max_work_items: 1,
                ..ExecutionLimits::GEOMETRY
            },
            ExecutionLimits {
                max_workspace_bytes: 1,
                ..ExecutionLimits::GEOMETRY
            },
        ] {
            let name = unit.clone();
            let value = factor.clone();
            let mut budget = PreparationBudget::new(ExecutionPolicy::new(limits).unwrap());
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let refused = profile.register_linear_unit_in_budget(name, value, &mut budget);
            let stats = window.close();
            assert!(matches!(
                refused,
                Err(GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. })
            ));
            assert_eq!(stats.allocations, 0);
            assert_eq!(stats.requested_bytes, 0);
            assert_eq!(profile.binding.get(), Some(&initial));
            assert_eq!(profile.linear_units(), []);
        }
        let policy = profile.policy().id();
        profile
            .register_linear_unit_in_budget(
                unit.clone(),
                factor.clone(),
                &mut PreparationBudget::new(generous),
            )
            .unwrap();
        let expected = profile.compute_binding_id();
        assert_eq!(profile.binding.get(), Some(&expected));
        assert_ne!(expected, initial);
        assert_eq!(profile.policy().id(), policy);
        let mut appended = Vec::new();
        profile.query_identity().append_to(&mut appended);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for _ in 0..32 {
            assert_eq!(profile.binding_id(), expected);
            assert_eq!(profile.query_identity().binding, expected);
            assert_eq!(profile.query_identity().framed_bytes().as_slice(), appended);
        }
        let stats = window.close();
        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.requested_bytes, 0);
        let mut cold = profile.clone();
        cold.binding = OnceLock::new();
        assert_eq!(cold, profile);
        assert_eq!(format!("{cold:?}"), format!("{profile:?}"));
        {
            #[derive(Debug)]
            #[expect(
                dead_code,
                reason = "The original derive-only Debug oracle consumes its fields through the generated formatter, which the dead-code analysis intentionally ignores"
            )]
            struct GeoProfile<'a> {
                references: &'a [ReferenceBinding],
                operations: &'a [OperationBinding],
                units: &'a [LinearUnitBinding],
                limits: ExecutionLimits,
            }
            let original = GeoProfile {
                references: profile.references(),
                operations: profile.operations(),
                units: profile.linear_units(),
                limits: profile.limits,
            };
            assert_eq!(format!("{profile:?}"), format!("{original:?}"));
            assert_eq!(format!("{profile:#?}"), format!("{original:#?}"));
        }
        profile
            .register_linear_unit_in_budget(unit, factor, &mut PreparationBudget::new(generous))
            .unwrap();
        assert_eq!(profile.binding.get(), Some(&expected));
        let changed = profile
            .with_limits(ExecutionLimits {
                max_work_items: 1,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap();
        assert_eq!(changed.binding.get(), Some(&expected));
        assert_ne!(changed.policy().id(), policy);
    }
}
