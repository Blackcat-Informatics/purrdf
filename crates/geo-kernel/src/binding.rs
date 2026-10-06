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
/// The immutable standard reference profile shared by borrowed query contexts.
pub static STANDARD_PROFILE: GeoProfile = GeoProfile::STANDARD;
static STANDARD_VOCABULARY: LazyLock<GeoVocab> = LazyLock::new(GeoVocab::standard);
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
    references: Vec<ReferenceBinding>,
    operations: Vec<OperationBinding>,
    units: Vec<LinearUnitBinding>,
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
        references: Vec::new(),
        operations: Vec::new(),
        units: Vec::new(),
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
        self.registration_in_budget(bytes, cost, budget, |profile| {
            profile.register_reference_admitted(crs, reference)
        })
    }

    fn register_reference_admitted(
        &mut self,
        crs: Crs,
        reference: GeographicReference,
    ) -> Result<(), GeoError> {
        if crs.as_str() == ogc::CRS84 {
            return if reference == *CRS84_REFERENCE {
                Ok(())
            } else {
                Err(GeoError::config(
                    "CRS84 always denotes WGS84 longitude/latitude",
                ))
            };
        }
        if crs.as_str() == ogc::EPSG4326
            && reference != CRS84_REFERENCE.clone().with_axes(AxisOrder::LatLon)
        {
            return Err(GeoError::config(
                "EPSG:4326 registration requires WGS84 with explicit latitude/longitude axes",
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
                    validate_endpoint(carrier, &reference, endpoint)?;
                }
            }
        }
        match self
            .references
            .binary_search_by(|entry| entry.crs.cmp(&crs))
        {
            Ok(index) if self.references[index].reference == reference => Ok(()),
            Ok(_) => Err(GeoError::config(format!(
                "conflicting reference registration for <{crs}>"
            ))),
            Err(index) => {
                self.references
                    .insert(index, ReferenceBinding { crs, reference });
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
        let cost = crate::numerical::rational_field_cost(&metres_per_unit)?.0;
        let bytes = (metres_per_unit.allocated_bytes() as u64)
            .checked_add(unit.retained_text_bytes() as u64)
            .and_then(|bytes| bytes.checked_add(size_of::<LinearUnitBinding>() as u64))
            .ok_or(GeoError::ArithmeticOverflow("unit registration storage"))?;
        self.registration_in_budget(bytes, cost, budget, |profile| {
            profile.register_linear_unit_admitted(unit, metres_per_unit)
        })
    }

    fn register_linear_unit_admitted(
        &mut self,
        unit: Crs,
        metres_per_unit: Rat,
    ) -> Result<(), GeoError> {
        if metres_per_unit.signum() <= 0 {
            return Err(GeoError::config(
                "linear output units require a positive exact metre factor",
            ));
        }
        if unit.as_str() == ogc::uom::METRE {
            return if metres_per_unit == Rat::one() {
                Ok(())
            } else {
                Err(GeoError::config("the official metre always has factor one"))
            };
        }
        match self.units.binary_search_by(|entry| entry.unit.cmp(&unit)) {
            Ok(index) if self.units[index].metres_per_unit == metres_per_unit => Ok(()),
            Ok(_) => Err(GeoError::config(
                "conflicting linear output-unit registration",
            )),
            Err(index) => {
                self.units.insert(
                    index,
                    LinearUnitBinding {
                        unit,
                        metres_per_unit,
                    },
                );
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
            .map_err(|_| GeoError::domain(format!("unregistered linear output unit <{unit}>")))
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
        self.registration_in_budget(bytes, cost, budget, |profile| {
            profile.register_operation_admitted(name, source, target, chain)
        })
    }

    fn register_operation_admitted(
        &mut self,
        name: Crs,
        source: Crs,
        target: Crs,
        chain: OperationChain,
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
            if let Ok(reference) = self.reference(carrier) {
                validate_endpoint(carrier, reference, endpoint)?;
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
            Ok(_) => Err(GeoError::config("conflicting operation name registration")),
            Err(index) => {
                self.operations.insert(index, binding);
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
        self.operations
            .binary_search_by(|entry| entry.name.cmp(name))
            .map(|index| &self.operations[index])
            .map_err(|_| GeoError::config(format!("unregistered coordinate operation <{name}>")))
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
        progress.exact(context, cost, || self.operation(name))
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
        progress.exact(context, cost, || self.operation_between(source, target))
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
                progress.exact(context, cost, || {
                    if source == binding.source() {
                        Ok(())
                    } else {
                        Err(GeoError::MissingOperation {
                            source: source.as_str().to_owned(),
                            target: binding.target().as_str().to_owned(),
                        })
                    }
                })?;
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
        let mut found: Option<&OperationChain> = None;
        for binding in &self.operations {
            if &binding.source == source && &binding.target == target {
                if let Some(previous) = found
                    && previous.id() != binding.chain.id()
                {
                    return Err(GeoError::config(
                        "multiple distinct coordinate chains; name the intended operation explicitly",
                    ));
                }
                found = Some(&binding.chain);
            }
        }
        found.ok_or_else(|| GeoError::MissingOperation {
            source: source.as_str().to_owned(),
            target: target.as_str().to_owned(),
        })
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
            return Err(GeoError::config(
                "metric reference requires a source carrier",
            ));
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
                        return Err(GeoError::config(
                            "multiple reachable physical metric surfaces; name the intended operation image explicitly",
                        ));
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
            .ok_or_else(|| GeoError::UnregisteredCrs(crs.as_str().to_owned()))
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
        progress.exact(context, cost, || self.reference(crs))
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
        progress.exact(context, cost, || self.projected_operation_reference(crs))
    }

    fn projected_operation_reference(
        &self,
        crs: &Crs,
    ) -> Result<crate::operation::OperationReference, GeoError> {
        use crate::operation::CoordinateUnit;
        if crs.as_str() == ogc::EPSG4326 {
            return Err(GeoError::UnregisteredCrs(crs.as_str().to_owned()));
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
                    return Err(GeoError::UnregisteredCrs(crs.as_str().to_owned()));
                }
                if declared.is_some_and(|previous| previous != endpoint) {
                    return Err(GeoError::config(
                        "conflicting operation carrier realizations, units or axes",
                    ));
                }
                declared = Some(endpoint);
            }
        }
        declared.ok_or_else(|| GeoError::UnregisteredCrs(crs.as_str().to_owned()))
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
                return Err(GeoError::config(
                    "chain endpoints disagree with registered actual carriers",
                ));
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
                    Some(&mut crate::numerical::ExactAdmission::new(
                        context,
                        &mut progress,
                    )),
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
        let result = Self::validate_literal_coordinates(
            literal,
            reference,
            Some(&mut crate::numerical::ExactAdmission::new(
                context,
                &mut progress,
            )),
        );
        context.release_workspace(workspace)?;
        result
    }

    fn validate_literal_coordinates(
        literal: &GeometryLiteral,
        reference: &GeographicReference,
        mut admission: Option<&mut crate::numerical::ExactAdmission<'_, '_>>,
    ) -> Result<(), GeoError> {
        for coordinate in literal.geometry().coords() {
            let (longitude, latitude) = match reference.axes() {
                AxisOrder::LonLat => (coordinate.x(), coordinate.y()),
                AxisOrder::LatLon => (coordinate.y(), coordinate.x()),
            };
            crate::numerical::exact_rational_counted(
                admission.as_deref_mut(),
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[longitude, latitude],
                4,
                || LonLat::validate_original(longitude, latitude),
            )??;
        }
        Ok(())
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
        let mut hash = blake3::Hasher::new();
        hash.update(PROFILE_BINDING_DOMAIN.as_bytes());
        frame_le_into(&mut hash, ogc::CRS84.as_bytes());
        frame_le_into(&mut hash, CRS84_REFERENCE.id().digest().as_bytes());
        for entry in &self.references {
            frame_le_into(&mut hash, entry.crs.as_str().as_bytes());
            frame_le_into(&mut hash, entry.reference.id().digest().as_bytes());
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
            frame_le_into(
                &mut hash,
                unit.metres_per_unit.numerator().to_string().as_bytes(),
            );
            frame_le_into(
                &mut hash,
                unit.metres_per_unit.denominator().to_string().as_bytes(),
            );
        }
        GeoBindingId::from_digest(Digest32::new(*hash.finalize().as_bytes()))
    }

    /// Copy an owned profile after admitting its complete original graph and
    /// the simultaneous copy. Shared operation chains remain shared; their
    /// complete graph allowance conservatively encloses the copied containers,
    /// strings and exact unit/reference fields. One logical copy unit per owned
    /// source byte bounds the original byte/limb copies and fixed field moves.
    ///
    /// # Errors
    /// Refuses cumulative configuration work/storage before any copy allocation.
    pub fn clone_in_budget(&self, budget: &mut crate::PreparationBudget) -> Result<Self, GeoError> {
        let before = budget.workspace_bytes();
        self.retain_configuration_in_budget(budget)?;
        let bytes = budget.workspace_bytes() - before;
        budget.retain(bytes, bytes)?;
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
        let mut bytes = (self.references.capacity() as u64)
            .checked_mul(size_of::<ReferenceBinding>() as u64)
            .and_then(|bytes| {
                bytes.checked_add((self.operations.capacity() as u64).checked_mul(size_of::<
                    OperationBinding,
                >(
                )
                    as u64)?)
            })
            .and_then(|bytes| {
                bytes.checked_add(
                    (self.units.capacity() as u64)
                        .checked_mul(size_of::<LinearUnitBinding>() as u64)?,
                )
            })
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
                .and_then(|bytes| bytes.checked_add(2 * size_of::<usize>() as u64))
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

    fn registration_in_budget<T>(
        &mut self,
        candidate_bytes: u64,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        budget: &mut crate::PreparationBudget,
        evaluate: impl FnOnce(&mut Self) -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
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
            let mut cost = cost
                .followed_by(binding)
                .and_then(|cost| {
                    cost.followed_by(purrdf_xsd::integer::ExactArithmeticCost::for_operation(
                        purrdf_xsd::integer::ExactOperation::Linear,
                        scan_bits,
                        8,
                    )?)
                })
                .ok_or(GeoError::ArithmeticOverflow("registration work"))?;
            // A growing table coexists with its prior allocation. Four initial
            // entries cover Vec's minimum allocation when the old table is empty.
            cost.workspace_bytes = cost
                .workspace_bytes
                .checked_add(graph_bytes)
                .and_then(|bytes| {
                    bytes.checked_add(
                        4 * (size_of::<ReferenceBinding>()
                            + size_of::<OperationBinding>()
                            + size_of::<LinearUnitBinding>()) as u64,
                    )
                })
                .ok_or(GeoError::ArithmeticOverflow("registration table growth"))?;
            phase.exact(cost, || {
                let result = evaluate(self)?;
                // The existing complete old-graph rendering plus candidate
                // rendering and byte scans admit this original binding body.
                // Later evaluator/host reads retain only its fixed-size value.
                let _ = self.binding_id();
                Ok(result)
            })
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
        self.retain_configuration_in_budget(budget)?;
        let cost = self.binding_cost(|| budget.retain(1, 0))?;
        let mut transient = *budget;
        transient.retain(cost.work_items, cost.workspace_bytes)?;
        budget.retain(cost.work_items, size_of::<GeoQueryIdentity>() as u64)?;
        Ok(self.query_identity_with_binding(self.binding_id()))
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
        progress.exact(context, cost, || Ok(self.binding_id()))
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

fn validate_endpoint(
    carrier: &Crs,
    reference: &GeographicReference,
    endpoint: crate::operation::OperationReference,
) -> Result<(), GeoError> {
    if endpoint.realization != reference.id().digest()
        || endpoint.unit != crate::operation::CoordinateUnit::Degrees
        || endpoint.swapped_axes != (reference.axes() == AxisOrder::LatLon)
    {
        return Err(GeoError::config(format!(
            "operation endpoint disagrees with declared carrier <{carrier}>"
        )));
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
        assert!(allocations.allocations > 0);
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
