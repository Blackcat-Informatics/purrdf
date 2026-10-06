// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Geographic mathematical laws, exact reference parameters and admission limits.
//!
//! Mathematical identities exclude implementation receipts and execution limits.
//! Reference bindings and execution policies have their own independently framed
//! identities. All rational parameters are normalized before hashing.

use purrdf_hash::{Domain, blake3, frame::frame_le_into, hex::Digest32};

use crate::{GeoError, Metres, Rat};

const LAW_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/semantic-law/v1");
const BINDING_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/reference-binding/v1");
const DATUM_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/native-datum/v1");
const POLICY_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/execution-policy/v1");

macro_rules! identity {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Digest32);

        impl $name {
            /// The content-addressed digest, rendered as 64 lowercase hex digits.
            #[must_use]
            pub const fn digest(self) -> Digest32 {
                self.0
            }
        }
    };
}

identity!(
    SemanticLawId,
    "Identity of a mathematical completed-output law."
);
identity!(
    GeoBindingId,
    "Identity of exact geographic reference bindings."
);
identity!(
    ExecutionPolicyId,
    "Identity of admitted precision and resource limits."
);

impl GeoBindingId {
    pub(crate) const fn from_digest(digest: Digest32) -> Self {
        Self(digest)
    }
}

impl SemanticLawId {
    pub(crate) const fn from_digest(digest: Digest32) -> Self {
        Self(digest)
    }
}

/// Mathematical point laws with fixed completed-output quantization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointLaw {
    /// True shortest ellipsoidal distance, correctly rounded half-even to a
    /// micrometre. An unresolved rounding boundary is a precision error.
    ShortestDistanceMicrometreV1,
    /// Direct endpoint coordinates correctly rounded half-even to 15 decimal
    /// degree places, with longitude in `[-180,180)` and zero at exact poles.
    DirectDegree15V1,
    /// Explicit direct output grid, with the same correctly half-even rounded
    /// endpoint and one-micrometre surface contract. Fifteen places coalesce
    /// with the standard direct law; precision never chooses this grid.
    DirectDecimalDegreeV1 {
        /// Caller-declared decimal degree places.
        decimal_places: u32,
    },
    /// Canonically selected shortest inverse branch, with correctly half-even
    /// quantized azimuth/arc degrees (15 places), reduced metres (6), scales
    /// (15), and signed square-metre area (2). Zero distance has absent
    /// azimuths; exact poles use the longitude-zero frame. Equal shortest
    /// branches select the smallest forward azimuth, then final azimuth.
    InverseMetadataV1,
}

impl PointLaw {
    /// This law's identity. Equivalent algorithms and tighter certificates do
    /// not change its preimage.
    #[must_use]
    pub fn id(self) -> SemanticLawId {
        if let Self::DirectDecimalDegreeV1 { decimal_places } = self {
            if decimal_places == 15 {
                return Self::DirectDegree15V1.id();
            }
            return SemanticLawId(hash_fields(
                LAW_DOMAIN,
                [
                    b"ellipsoid-direct;half-even;explicit-decimal-degree-grid;longitude=[-180,180);exact-pole-longitude=0;surface-bound-metres=0.000001;certificate=v1;unresolved=precision-error".as_slice(),
                    &decimal_places.to_be_bytes(),
                ],
            ));
        }
        let descriptor: &[u8] = match self {
            Self::ShortestDistanceMicrometreV1 => b"true-shortest-ellipsoid-distance;half-even;metre-quantum=0.000001;response-uncertainty=0.0000005;host-conversion=half-ulp;certificate=v1;unresolved=precision-error",
            Self::DirectDegree15V1 => b"ellipsoid-direct;half-even;degree-quantum=0.000000000000001;longitude=[-180,180);exact-pole-longitude=0;unresolved=precision-error",
            Self::InverseMetadataV1 => b"true-shortest-ellipsoid-inverse;branch=min-forward-azimuth-then-final;azimuth=[0,360);exact-pole-frame-longitude=0;zero=absent-azimuths-zero-arc-reduced-area-unit-scales;half-even;azimuth-arc-degree-quantum=0.000000000000001;reduced-metre-quantum=0.000001;scale-quantum=0.000000000000001;area-square-metre-quantum=0.01;geodesic-quadrilateral-area=+integral-Q-dlongitude;pole-cuts=explicit-canonical-[-180,180);distance-law=true-shortest-micrometre-v1;certificate=v1;unresolved=precision-error",
            Self::DirectDecimalDegreeV1 { .. } => unreachable!("explicit grids handled above"),
        };
        SemanticLawId(hash_fields(LAW_DOMAIN, [descriptor]))
    }
}

/// Evidence about how an answer was produced, independent of its semantic law.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImplementationReceipt {
    /// Content identity of the implementation being qualified.
    pub implementation: Digest32,
    /// Content identity of the coefficient/constants generator.
    pub generator: Digest32,
    /// Content identity of the proof artifact.
    pub proof: Digest32,
    /// Content identity of the captured compiler/toolchain descriptor.
    pub compiler: Digest32,
    /// Content identity of the selected execution-backend descriptor.
    pub backend: Digest32,
}

/// The axis order a geographic carrier declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisOrder {
    /// Longitude followed by latitude, as in OGC CRS84.
    LonLat,
    /// Latitude followed by longitude, as in an explicit EPSG:4326 binding.
    LatLon,
}

/// An exact oblate ellipsoid prepared independently of any datum transformation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreparedEllipsoid {
    semimajor: Rat,
    inverse_flattening: Rat,
    semiminor: Rat,
    normal_minimum: Metres,
    normal_maximum: Metres,
}

impl PreparedEllipsoid {
    /// Prepare positive metre axes from exact semimajor axis and inverse
    /// flattening. The supported oblate domain is `a>0` and `1/f>1`.
    ///
    /// # Errors
    ///
    /// Refuses nonpositive axes and a flattening outside the oblate domain.
    pub fn new(semimajor: Rat, inverse_flattening: Rat) -> Result<Self, GeoError> {
        Self::new_in_budget(
            semimajor,
            inverse_flattening,
            &mut crate::PreparationBudget::new(ExecutionPolicy::geometry()),
        )
    }

    /// Prepare exact axes and immutable metric factors under the same cumulative
    /// configuration admission as reference and operation compilation.
    ///
    /// # Errors
    /// Refuses invalid axes and limits before each exact derived operation.
    pub fn new_in_budget(
        semimajor: Rat,
        inverse_flattening: Rat,
        budget: &mut crate::PreparationBudget,
    ) -> Result<Self, GeoError> {
        use purrdf_xsd::integer::ExactOperation as Op;
        let source_bytes = (semimajor.allocated_bytes() as u64)
            .checked_add(inverse_flattening.allocated_bytes() as u64)
            .and_then(|bytes| bytes.checked_add(size_of::<Self>() as u64))
            .ok_or(GeoError::ArithmeticOverflow("ellipsoid source storage"))?;
        budget.with_workspace(source_bytes, |budget| {
            let zero = Rat::zero();
            let one = Rat::one();
            budget.exact_rationals(Op::RationalCompare, &[&semimajor, &zero], 1, || {
                if semimajor <= zero {
                    Err(GeoError::InvalidEllipsoid(
                        "semimajor axis must be positive",
                    ))
                } else {
                    Ok(())
                }
            })?;
            budget.exact_rationals(Op::RationalCompare, &[&inverse_flattening, &one], 1, || {
                if inverse_flattening <= one {
                    Err(GeoError::InvalidEllipsoid(
                        "inverse flattening must exceed one",
                    ))
                } else {
                    Ok(())
                }
            })?;
            let flattening =
                budget.exact_rationals(Op::Linear, &[&inverse_flattening], 2, || {
                    Ok(inverse_flattening.recip().expect("validated positive"))
                })?;
            budget.retain(0, flattening.allocated_bytes() as u64)?;
            let remainder =
                budget.exact_rationals(Op::RationalAdd, &[&one, &flattening], 1, || {
                    Ok(one.sub(&flattening))
                })?;
            budget.retain(0, remainder.allocated_bytes() as u64)?;
            let semiminor = budget.exact_rationals(
                Op::RationalMultiply,
                &[&semimajor, &remainder],
                1,
                || Ok(semimajor.mul(&remainder)),
            )?;
            budget.retain(0, semiminor.allocated_bytes() as u64)?;
            let minor_squared = budget.exact_rationals(
                Op::RationalMultiply,
                &[&semiminor, &semiminor],
                1,
                || Ok(semiminor.mul(&semiminor)),
            )?;
            budget.retain(0, minor_squared.allocated_bytes() as u64)?;
            let minimum = budget.exact_rationals(
                Op::RationalDivide,
                &[&minor_squared, &semimajor],
                1,
                || Ok(minor_squared.div(&semimajor).expect("positive axis")),
            )?;
            budget.retain(0, minimum.allocated_bytes() as u64)?;
            let major_squared = budget.exact_rationals(
                Op::RationalMultiply,
                &[&semimajor, &semimajor],
                1,
                || Ok(semimajor.mul(&semimajor)),
            )?;
            budget.retain(0, major_squared.allocated_bytes() as u64)?;
            let maximum = budget.exact_rationals(
                Op::RationalDivide,
                &[&major_squared, &semiminor],
                1,
                || Ok(major_squared.div(&semiminor).expect("positive axis")),
            )?;
            Ok(Self {
                semimajor: budget.share_rational(semimajor)?,
                inverse_flattening: budget.share_rational(inverse_flattening)?,
                semiminor: budget.share_rational(semiminor)?,
                normal_minimum: Metres::new(budget.share_rational(minimum)?),
                normal_maximum: Metres::new(budget.share_rational(maximum)?),
            })
        })
    }

    /// Native WGS84 ellipsoid: `a=6378137 m`, `1/f=298.257223563` exactly.
    #[must_use]
    pub fn wgs84() -> Self {
        Self::native("298.257223563")
    }

    /// Native CGCS2000 ellipsoid: `a=6378137 m`, `1/f=298.257222101` exactly.
    #[must_use]
    pub fn cgcs2000() -> Self {
        Self::native("298.257222101")
    }

    fn native(inverse_flattening: &str) -> Self {
        Self::new(
            Rat::from_i64(6_378_137),
            Rat::parse_decimal(inverse_flattening).expect("a frozen exact decimal"),
        )
        .expect("a positive native oblate ellipsoid")
    }

    /// Exact semimajor axis in metres.
    #[must_use]
    pub const fn semimajor(&self) -> &Rat {
        &self.semimajor
    }

    /// Exact semiminor axis in metres.
    #[must_use]
    pub const fn semiminor(&self) -> &Rat {
        &self.semiminor
    }

    /// Exact inverse flattening.
    #[must_use]
    pub const fn inverse_flattening(&self) -> &Rat {
        &self.inverse_flattening
    }

    /// Exact flattening `f`.
    #[must_use]
    pub fn flattening(&self) -> Rat {
        self.inverse_flattening.recip().expect("validated positive")
    }

    /// Exact squared eccentricity `f(2-f)`.
    #[must_use]
    pub fn eccentricity_squared(&self) -> Rat {
        let flattening = self.flattening();
        flattening.mul(&Rat::from_i64(2).sub(&flattening))
    }

    /// Exact global lower and upper metric factors for geodetic-normal angles.
    /// Multiplying normal-sphere path length by these factors encloses its
    /// ellipsoidal surface length. The axes are positive and oblate.
    #[must_use]
    pub fn normal_metric_bounds(&self) -> (Metres, Metres) {
        (self.normal_minimum.clone(), self.normal_maximum.clone())
    }

    /// Borrow the exact metric factors computed once with the immutable axes.
    #[must_use]
    pub const fn normal_metric_bounds_ref(&self) -> (&Metres, &Metres) {
        (&self.normal_minimum, &self.normal_maximum)
    }

    /// Allocated exact limbs owned by the axes and their prepared metric factors.
    /// The enclosing object's fixed size is accounted by its actual owner.
    #[must_use]
    pub fn retained_limb_bytes(&self) -> u64 {
        [
            &self.semimajor,
            &self.inverse_flattening,
            &self.semiminor,
            self.normal_minimum.exact(),
            self.normal_maximum.exact(),
        ]
        .into_iter()
        .fold(0_u64, |bytes, value| {
            bytes.saturating_add(value.allocated_bytes() as u64)
        })
    }
}

/// A declared geographic reference, without an inferred datum transformation.
#[derive(Clone)]
pub struct GeographicReference {
    ellipsoid: PreparedEllipsoid,
    datum: Digest32,
    axes: AxisOrder,
    // Derived immutable metadata. It never participates in reference equality
    // or hashing, and each axis binding computes exactly the original preimage.
    identity: std::sync::OnceLock<GeoBindingId>,
}

#[expect(
    clippy::missing_fields_in_debug,
    reason = "Derived identity cache is excluded so cold and warm references retain the original identical Debug value"
)]
impl core::fmt::Debug for GeographicReference {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("GeographicReference")
            .field("ellipsoid", &self.ellipsoid)
            .field("datum", &self.datum)
            .field("axes", &self.axes)
            .finish()
    }
}

impl PartialEq for GeographicReference {
    fn eq(&self, other: &Self) -> bool {
        self.ellipsoid == other.ellipsoid && self.datum == other.datum && self.axes == other.axes
    }
}
impl Eq for GeographicReference {}
impl core::hash::Hash for GeographicReference {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.ellipsoid.hash(state);
        self.datum.hash(state);
        self.axes.hash(state);
    }
}

impl GeographicReference {
    /// Bind a caller-supplied datum identity, ellipsoid and carrier axis order.
    #[must_use]
    pub const fn new(ellipsoid: PreparedEllipsoid, datum: Digest32, axes: AxisOrder) -> Self {
        Self {
            ellipsoid,
            datum,
            axes,
            identity: std::sync::OnceLock::new(),
        }
    }

    /// Native WGS84 longitude/latitude reference.
    #[must_use]
    pub fn wgs84() -> Self {
        Self::native(PreparedEllipsoid::wgs84(), b"WGS84")
    }

    /// Native CGCS2000 longitude/latitude reference. No WGS84 identity
    /// operation is installed by this constructor.
    #[must_use]
    pub fn cgcs2000() -> Self {
        Self::native(PreparedEllipsoid::cgcs2000(), b"CGCS2000")
    }

    fn native(ellipsoid: PreparedEllipsoid, datum: &[u8]) -> Self {
        Self::new(
            ellipsoid,
            hash_fields(DATUM_DOMAIN, [datum]),
            AxisOrder::LonLat,
        )
    }

    /// The exact ellipsoid parameters.
    #[must_use]
    pub const fn ellipsoid(&self) -> &PreparedEllipsoid {
        &self.ellipsoid
    }

    /// The explicitly declared datum identity.
    #[must_use]
    pub const fn datum(&self) -> Digest32 {
        self.datum
    }

    /// Whether two explicitly declared references use the same physical
    /// ellipsoid and datum. Carrier axis order is interpreted separately.
    /// This compares declarations; it never supplies a datum operation.
    #[must_use]
    pub fn same_surface(&self, other: &Self) -> bool {
        self.datum == other.datum && self.ellipsoid == other.ellipsoid
    }

    /// The axis order of coordinates at the carrier boundary.
    #[must_use]
    pub const fn axes(&self) -> AxisOrder {
        self.axes
    }

    /// Declare a different carrier axis order without changing coordinates.
    #[must_use]
    pub fn with_axes(mut self, axes: AxisOrder) -> Self {
        if self.axes != axes {
            self.axes = axes;
            self.identity = std::sync::OnceLock::new();
        }
        self
    }

    /// Identity of the exact parameters, datum and axis binding.
    #[must_use]
    pub fn id(&self) -> GeoBindingId {
        *self.identity.get_or_init(|| self.compute_id())
    }

    /// Read an identity previously produced by its admitted original worker.
    /// This never renders or initializes a cold reference.
    pub(crate) fn cached_identity(&self) -> Option<GeoBindingId> {
        self.identity.get().copied()
    }

    fn compute_id(&self) -> GeoBindingId {
        let axes = match self.axes {
            AxisOrder::LonLat => b"longitude-latitude".as_slice(),
            AxisOrder::LatLon => b"latitude-longitude".as_slice(),
        };
        let a_num = self.ellipsoid.semimajor().numerator().to_string();
        let a_den = self.ellipsoid.semimajor().denominator().to_string();
        let f_num = self.ellipsoid.inverse_flattening().numerator().to_string();
        let f_den = self
            .ellipsoid
            .inverse_flattening()
            .denominator()
            .to_string();
        GeoBindingId(hash_fields(
            BINDING_DOMAIN,
            [
                b"geographic-reference".as_slice(),
                self.datum.as_bytes(),
                axes,
                b"degrees;metres",
                a_num.as_bytes(),
                a_den.as_bytes(),
                f_num.as_bytes(),
                f_den.as_bytes(),
            ],
        ))
    }
}

/// Checked resource admission for a complete geographic operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExecutionLimits {
    /// Complete output vertices/elements allowed.
    pub max_output_elements: u64,
    /// Internal work items allowed.
    pub max_work_items: u64,
    /// Retained workspace bytes allowed.
    pub max_workspace_bytes: u64,
    /// Solver iterations allowed.
    pub max_iterations: u32,
    /// Subdivision levels allowed.
    pub max_subdivision_levels: u32,
    /// Highest admitted numerical precision.
    pub max_precision_bits: u32,
    /// Reusable integer destinations admitted for one worker's live enclosures.
    pub max_scratch_destinations: u32,
}

impl ExecutionLimits {
    /// The fixed geometry admission defaults.
    pub const GEOMETRY: Self = Self {
        max_output_elements: 131_072,
        max_work_items: 262_144,
        max_workspace_bytes: 64 * 1024 * 1024,
        max_iterations: 128,
        max_subdivision_levels: 64,
        max_precision_bits: 512,
        max_scratch_destinations: 768,
    };
}

/// A validated execution policy. It never changes a completed output law.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExecutionPolicy {
    limits: ExecutionLimits,
    id: ExecutionPolicyId,
}

impl ExecutionPolicy {
    /// Validate explicitly supplied limits before numerical work or allocation.
    ///
    /// # Errors
    ///
    /// Zero admission limits are refused. A positive but insufficient policy
    /// remains valid and produces the corresponding operational exhaustion.
    pub fn new(limits: ExecutionLimits) -> Result<Self, GeoError> {
        if limits.max_output_elements == 0
            || limits.max_work_items == 0
            || limits.max_workspace_bytes == 0
            || limits.max_iterations == 0
            || limits.max_subdivision_levels == 0
            || limits.max_precision_bits == 0
            || limits.max_scratch_destinations == 0
        {
            return Err(GeoError::InvalidExecutionPolicy(
                "admission limits must be positive",
            ));
        }
        let fields = [
            limits.max_output_elements,
            limits.max_work_items,
            limits.max_workspace_bytes,
            u64::from(limits.max_iterations),
            u64::from(limits.max_subdivision_levels),
            u64::from(limits.max_precision_bits),
            u64::from(limits.max_scratch_destinations),
        ]
        .map(u64::to_le_bytes);
        let id = ExecutionPolicyId(hash_fields(
            POLICY_DOMAIN,
            fields.iter().map(<[u8; 8]>::as_slice),
        ));
        Ok(Self { limits, id })
    }

    /// Fixed default geometry admission.
    #[must_use]
    pub fn geometry() -> Self {
        Self::new(ExecutionLimits::GEOMETRY).expect("positive frozen limits")
    }

    /// The checked limits, separate from the mathematical law.
    #[must_use]
    pub const fn limits(&self) -> &ExecutionLimits {
        &self.limits
    }

    /// Admit a following phase after actual prior work and retained storage.
    /// The original caller policy remains unchanged; mathematical identities
    /// never depend on this temporary child admission.
    ///
    /// # Errors
    /// Refuses consumed or overflowing work/workspace with the original limit.
    pub fn remaining_after(self, work: u64, workspace: u64) -> Result<Self, GeoError> {
        let mut limits = self.limits;
        limits.max_work_items = limits
            .max_work_items
            .checked_sub(work)
            .filter(|remaining| *remaining > 0)
            .ok_or(GeoError::WorkExhausted {
                limit: self.limits.max_work_items,
            })?;
        limits.max_workspace_bytes = limits
            .max_workspace_bytes
            .checked_sub(workspace)
            .filter(|remaining| *remaining > 0)
            .ok_or(GeoError::MemoryExhausted {
                limit: self.limits.max_workspace_bytes,
            })?;
        Self::new(limits)
    }

    /// Identity of this admission policy only.
    #[must_use]
    pub const fn id(self) -> ExecutionPolicyId {
        self.id
    }
}

/// Hash an injectively framed descriptor without materializing its preimage.
pub(crate) fn hash_fields<'a>(
    domain: Domain,
    fields: impl IntoIterator<Item = &'a [u8]>,
) -> Digest32 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    for field in fields {
        frame_le_into(&mut hasher, field);
    }
    Digest32::new(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::{
        AxisOrder, ExecutionLimits, ExecutionPolicy, GeographicReference, PointLaw,
        PreparedEllipsoid,
    };
    use crate::{GeoError, Int, Rat};
    use purrdf_hash::hex::Digest32;

    /// One completed distance: its profile identity, certificate and value.
    fn completed(
        reference: &GeographicReference,
        limits: ExecutionLimits,
        backend: Option<purrdf_xsd::math::FloatProductBackend>,
        prepared: bool,
        pair: &(crate::LonLat, crate::LonLat),
    ) -> (super::GeoBindingId, Vec<u8>, Rat) {
        let policy = ExecutionPolicy::new(limits).unwrap();
        let mut context = crate::MetricContext::new(reference.clone(), policy).unwrap();
        if let Some(backend) = backend {
            context.set_binary64_backend(backend).unwrap();
        }
        let geodesic = if prepared {
            crate::PreparedGeodesic::prepare(reference.clone(), &mut context).unwrap()
        } else {
            crate::PreparedGeodesic::new(reference.clone())
        };
        let estimate = geodesic.distance(&pair.0, &pair.1, &mut context).unwrap();
        (
            reference.id(),
            estimate.certificate_bytes(),
            estimate.value().exact().clone(),
        )
    }

    fn pair() -> (crate::LonLat, crate::LonLat) {
        let point = |longitude: &str, latitude: &str| {
            crate::LonLat::new(
                Rat::parse_decimal(longitude).unwrap(),
                Rat::parse_decimal(latitude).unwrap(),
            )
            .unwrap()
        };
        // Asymmetric, so reading the carrier as latitude-longitude moves it.
        (point("10", "20"), point("31.5", "-12.25"))
    }

    /// The distance profile identity binds exactly the declarations a completed
    /// answer depends on. Contract (a), correct rounding of the true distance,
    /// makes every proof and execution choice identity-free: they cannot change
    /// a completed certificate, so they must not change the identity either.
    #[test]
    fn distance_profile_identity_changes_exactly_when_completed_outputs_can_change() {
        use purrdf_hash::Backend as _;
        let pair = pair();
        let wgs84 = GeographicReference::wgs84();
        let base = completed(&wgs84, ExecutionLimits::GEOMETRY, None, true, &pair);

        // Only if: identity-free execution changes keep identity and bytes.
        let mut variations = vec![
            completed(&wgs84, ExecutionLimits::GEOMETRY, None, false, &pair),
            completed(
                &wgs84,
                ExecutionLimits {
                    max_work_items: 1 << 40,
                    max_precision_bits: 4096,
                    max_iterations: 1024,
                    ..ExecutionLimits::GEOMETRY
                },
                None,
                true,
                &pair,
            ),
            completed(
                &wgs84,
                ExecutionLimits {
                    max_precision_bits: 64,
                    ..ExecutionLimits::GEOMETRY
                },
                None,
                true,
                &pair,
            ),
        ];
        for backend in purrdf_xsd::math::FloatProductBackend::all_available() {
            variations.push(completed(
                &wgs84,
                ExecutionLimits::GEOMETRY,
                Some(backend),
                true,
                &pair,
            ));
        }
        let plain = crate::geodesic::distance(pair.0.clone(), pair.1.clone()).unwrap();
        variations.push((
            wgs84.id(),
            plain.certificate_bytes(),
            plain.value().exact().clone(),
        ));
        for variation in &variations {
            assert_eq!(variation, &base);
        }

        // If: every identity field has a witness whose completed bytes change.
        let numeric = [
            (
                "semimajor axis",
                GeographicReference::new(
                    PreparedEllipsoid::new(
                        Rat::from_i64(6_378_138),
                        wgs84.ellipsoid().inverse_flattening().clone(),
                    )
                    .unwrap(),
                    wgs84.datum(),
                    AxisOrder::LonLat,
                ),
            ),
            (
                "inverse flattening",
                GeographicReference::new(
                    PreparedEllipsoid::new(
                        wgs84.ellipsoid().semimajor().clone(),
                        Rat::from_i64(298),
                    )
                    .unwrap(),
                    wgs84.datum(),
                    AxisOrder::LonLat,
                ),
            ),
            ("native realization", GeographicReference::cgcs2000()),
        ];
        for (field, reference) in numeric {
            let changed = completed(&reference, ExecutionLimits::GEOMETRY, None, true, &pair);
            assert_ne!(changed.0, base.0, "{field} identity");
            assert_ne!(changed.1, base.1, "{field} certificate");
            assert_ne!(changed.2, base.2, "{field} value");
        }
        // Datum and carrier axes are declared semantics, not numeric
        // parameters. The same ellipsoid on another datum names different
        // physical points, and the axis order decides how a carrier literal's
        // two numbers become longitude and latitude (the EPSG:4326 swap is
        // pinned in the SPARQL standard-function tests). Logical LonLat input
        // has the same arithmetic, so the value is equal, while the completed
        // certificate carrying the binding changes with its identity.
        for reference in [
            GeographicReference::new(
                wgs84.ellipsoid().clone(),
                Digest32::new([42; 32]),
                AxisOrder::LonLat,
            ),
            wgs84.clone().with_axes(AxisOrder::LatLon),
        ] {
            let changed = completed(&reference, ExecutionLimits::GEOMETRY, None, true, &pair);
            assert_ne!(changed.0, base.0);
            assert_ne!(changed.1, base.1);
            assert_eq!(changed.2, base.2);
        }
        // Frozen vector pinned to its profile identity.
        assert_eq!(base.2, Rat::parse_decimal(FROZEN_WGS84_DISTANCE).unwrap());
        assert_eq!(base.0.digest().to_string(), FROZEN_WGS84_BINDING);
    }

    const FROZEN_WGS84_DISTANCE: &str = "4274723.68913";
    const FROZEN_WGS84_BINDING: &str =
        "049197c9c642443788358e9e8daa536c50f6a6cf35fc34977c4f213d1cb6b7f0";

    /// Grid profile identity changes exactly when an assigned key can change.
    #[test]
    fn grid_profile_identity_changes_exactly_when_assigned_keys_can_change() {
        use crate::cells::{CubeHilbertQ62V1, NativeGridProfile};
        let wgs84 = CubeHilbertQ62V1::new(NativeGridProfile::Wgs84);
        let cgcs2000 = CubeHilbertQ62V1::new(NativeGridProfile::Cgcs2000);
        assert_ne!(wgs84.profile_id(), cgcs2000.profile_id());
        assert_eq!(
            wgs84.profile_id(),
            CubeHilbertQ62V1::new(NativeGridProfile::Wgs84).profile_id()
        );
        let points: Vec<crate::LonLat> = (0..400)
            .map(|step| {
                crate::LonLat::new(
                    Rat::new(Int::from_i64(step * 9 - 1800), Int::from_i64(10)).unwrap(),
                    Rat::new(Int::from_i64(step * 37 % 1801 - 900), Int::from_i64(10)).unwrap(),
                )
                .unwrap()
            })
            .collect();
        // Only if: batch and scalar assignment are the same law.
        let mut batch = vec![wgs84.assign(&points[0], 30).unwrap(); points.len()];
        wgs84.assign_batch(&points, 30, &mut batch).unwrap();
        for (point, key) in points.iter().zip(&batch) {
            assert_eq!(wgs84.assign(point, 30).unwrap(), *key);
        }
        // If: assignment uses geodetic normals, so the two native
        // realizations share every local key; their identities still differ
        // because the profile's other completed outputs depend on the exact
        // ellipsoid: certified physical cell scales and metric disk covers.
        for point in &points {
            assert_eq!(
                wgs84.assign(point, 30).unwrap().key(),
                cgcs2000.assign(point, 30).unwrap().key()
            );
        }
        let scales = |grid: CubeHilbertQ62V1| {
            let bounds = grid.physical_scale_bounds(16).unwrap();
            (
                bounds.lower().exact().clone(),
                bounds.upper().exact().clone(),
            )
        };
        assert_ne!(scales(wgs84), scales(cgcs2000));
    }

    #[test]
    fn large_prepared_ellipsoid_clones_share_all_original_and_derived_limbs() {
        let scale = Int::one().shl(256);
        let a = Rat::from_int(scale.add(&Int::one()));
        let f = Rat::new(scale.mul(&Int::from_i64(300)).add(&Int::one()), scale).unwrap();
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 128_000_000,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut budget = crate::PreparationBudget::new(policy);
        let ellipsoid = PreparedEllipsoid::new_in_budget(a, f, &mut budget).unwrap();
        let retained = ellipsoid.retained_limb_bytes();
        assert!(retained > 0);
        let reference =
            GeographicReference::new(ellipsoid, Digest32::new([7; 32]), AxisOrder::LonLat);
        let binding = reference.id();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for _ in 0..64 {
            let copy = std::hint::black_box(reference.clone());
            let bounds = std::hint::black_box(copy.ellipsoid().normal_metric_bounds());
            std::hint::black_box((&copy, &bounds));
        }
        let measurement = window.close();
        assert_eq!(measurement.allocations, 0);
        assert_eq!(measurement.requested_bytes, 0);
        assert_eq!(reference.id(), binding);
        assert_eq!(reference.ellipsoid().retained_limb_bytes(), retained);
    }

    #[test]
    fn equivalent_rational_bindings_have_one_identity() {
        let reference = GeographicReference::wgs84();
        let a =
            Rat::new(Int::from_i64(12_756_274), Int::from_i64(2)).expect("positive denominator");
        let f = Rat::parse_decimal("298.2572235630").expect("decimal");
        let equivalent = GeographicReference::new(
            PreparedEllipsoid::new(a, f).expect("native ellipsoid"),
            reference.datum(),
            AxisOrder::LonLat,
        );
        assert_eq!(reference.id(), equivalent.id());
        assert_ne!(
            reference.id(),
            reference.clone().with_axes(AxisOrder::LatLon).id()
        );
        assert_ne!(reference.id(), GeographicReference::cgcs2000().id());
        let different_datum = GeographicReference::new(
            reference.ellipsoid().clone(),
            Digest32::new([42; 32]),
            AxisOrder::LonLat,
        );
        assert_ne!(reference.id(), different_datum.id());
    }

    #[test]
    fn reference_cache_preserves_value_hash_debug_and_exact_framing() {
        use core::hash::{Hash, Hasher};
        let cold = GeographicReference::wgs84();
        let warm = cold.clone();
        let expected = warm.compute_id();
        let debug = format!("{warm:?}");
        assert_eq!(warm.id(), expected);
        assert_eq!(warm.id(), expected);
        assert_eq!(cold, warm);
        assert_eq!(debug, format!("{warm:?}"));
        let digest = |reference: &GeographicReference| {
            let mut hasher = purrdf_hash::fixed::FixedHasher::default();
            reference.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(digest(&cold), digest(&warm));
        let lat_lon = warm.clone().with_axes(AxisOrder::LatLon);
        assert_eq!(lat_lon.id(), lat_lon.compute_id());
        assert_ne!(warm.id(), lat_lon.id());
        assert_eq!(warm.id(), warm.with_axes(AxisOrder::LonLat).id());
    }

    #[test]
    fn point_law_and_reference_do_not_bind_admission() {
        let law = PointLaw::ShortestDistanceMicrometreV1.id();
        let binding = GeographicReference::wgs84().id();
        let default = ExecutionPolicy::geometry();
        let mut raised = ExecutionLimits::GEOMETRY;
        raised.max_precision_bits *= 2;
        raised.max_work_items *= 2;
        let raised = ExecutionPolicy::new(raised).expect("positive limits");
        assert_ne!(default.id(), raised.id());
        assert_eq!(law, PointLaw::ShortestDistanceMicrometreV1.id());
        assert_eq!(binding, GeographicReference::wgs84().id());
        assert_ne!(law, PointLaw::DirectDegree15V1.id());
    }

    #[test]
    fn insufficient_positive_precision_is_admitted_as_a_policy() {
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_precision_bits = 1;
        assert!(ExecutionPolicy::new(limits).is_ok());
        limits.max_precision_bits = 0;
        assert!(matches!(
            ExecutionPolicy::new(limits),
            Err(GeoError::InvalidExecutionPolicy(_))
        ));
    }
}
