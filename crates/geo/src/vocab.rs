// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The caller-supplied GeoSPARQL vocabulary — the only place this crate learns an
//! IRI.
//!
//! GeoSPARQL's IRIs belong to OGC, not to PurRDF, and PurRDF is not an ontology:
//! it mints no vocabulary IRIs and ships no fabricated defaults. So [`GeoVocab`]
//! has **no `Default` impl and never will**. A host that wants GeoSPARQL builds
//! one from the standard's own namespaces and passes it in; a feature whose
//! vocabulary term is absent is a hard error naming the missing term, never a
//! guess.
//!
//! # Namespaces, not term-by-term configuration
//!
//! A caller supplies two namespaces (`geo:` and `geof:`) and two coordinate
//! reference systems, not seventy-four individual IRIs. That is the honest line:
//! the *local names* — `sfWithin`, `wktLiteral`, `hasDefaultGeometry` — are fixed
//! by OGC 22-047r1 and are no more PurRDF's invention than the string `"SELECT"`
//! is, while the *namespace* is the part a deployment could legitimately differ
//! on and is therefore the part this crate refuses to assume. Nothing is
//! fabricated: with no namespace supplied there is no vocabulary, and every
//! registration surface is inert.
//!
//! # Units are declared, not derived
//!
//! `geof:distance`, `geof:area`, `geof:length` and `geof:perimeter` take a units
//! IRI, and the `metric*` family fixes that unit to the metre. Answering either
//! honestly requires knowing what one coordinate unit of a geometry's coordinate
//! reference system *is*, which is a property of the CRS — and this crate ships
//! no coordinate-reference-system database (see [`crate::geom::Crs`]: it
//! reprojects nothing). So the caller **declares** the linear unit of each CRS it
//! uses, with [`GeoVocabBuilder::declare_crs_unit`], and a measurement asked in a
//! unit that has not been declared for that CRS is refused by name rather than
//! answered in the wrong unit. A number in the wrong unit is the worst possible
//! answer: it is plausible, it is silent, and it is off by a factor nobody can
//! see.

use crate::error::GeoError;
use crate::geom::Crs;

/// A term of the GeoSPARQL core (`geo:`) vocabulary.
///
/// Carries only the local name; the namespace comes from [`GeoVocab`]. Every
/// local name here is quoted from OGC 22-047r1 Clauses 8 and 10 and cross-checked
/// against the shipped `geo.ttl` ontology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GeoTerm {
    // ---- literal datatypes (Clause 10.8) ----
    /// `geo:wktLiteral`.
    WktLiteral,
    /// `geo:gmlLiteral`.
    GmlLiteral,
    /// `geo:geoJSONLiteral`.
    GeoJsonLiteral,
    /// `geo:kmlLiteral`.
    KmlLiteral,
    /// `geo:dggsLiteral`.
    DggsLiteral,

    // ---- classes (Clause 8.2) ----
    /// `geo:SpatialObject`.
    SpatialObject,
    /// `geo:Feature`.
    Feature,
    /// `geo:Geometry`.
    Geometry,
    /// `geo:SpatialObjectCollection`.
    SpatialObjectCollection,
    /// `geo:FeatureCollection`.
    FeatureCollection,
    /// `geo:GeometryCollection`.
    GeometryCollection,

    // ---- feature properties (Requirement 7) ----
    /// `geo:hasGeometry`.
    HasGeometry,
    /// `geo:hasDefaultGeometry` — the property the Query Rewrite rules
    /// dereference, and the only one they dereference.
    HasDefaultGeometry,
    /// `geo:defaultGeometry` — the GeoSPARQL 1.0 legacy alias the ontology keeps
    /// as `owl:equivalentProperty` of [`Self::HasDefaultGeometry`]. Accepted on
    /// input; never emitted.
    DefaultGeometry,
    /// `geo:hasBoundingBox`.
    HasBoundingBox,
    /// `geo:hasCentroid`.
    HasCentroid,

    // ---- serialization properties (Clause 10.8) ----
    /// `geo:hasSerialization`.
    HasSerialization,
    /// `geo:asWKT`.
    AsWkt,
    /// `geo:asGML`.
    AsGml,
    /// `geo:asGeoJSON`.
    AsGeoJson,
    /// `geo:asKML`.
    AsKml,
    /// `geo:asDGGS`.
    AsDggs,

    // ---- geometry properties (Requirement 13) ----
    /// `geo:dimension`.
    Dimension,
    /// `geo:coordinateDimension`.
    CoordinateDimension,
    /// `geo:spatialDimension`.
    SpatialDimension,
    /// `geo:isEmpty`.
    IsEmpty,
    /// `geo:isSimple`.
    IsSimple,
    /// `geo:hasSpatialResolution`.
    HasSpatialResolution,
    /// `geo:hasMetricSpatialResolution`.
    HasMetricSpatialResolution,
    /// `geo:hasSpatialAccuracy`.
    HasSpatialAccuracy,
    /// `geo:hasMetricSpatialAccuracy`.
    HasMetricSpatialAccuracy,

    // ---- size properties (Requirement 6) ----
    /// `geo:hasSize`.
    HasSize,
    /// `geo:hasMetricSize`.
    HasMetricSize,
    /// `geo:hasLength`.
    HasLength,
    /// `geo:hasMetricLength`.
    HasMetricLength,
    /// `geo:hasPerimeterLength` — note the name: there is no `geo:hasPerimeter`,
    /// even though the *function* is `geof:perimeter`.
    HasPerimeterLength,
    /// `geo:hasMetricPerimeterLength`.
    HasMetricPerimeterLength,
    /// `geo:hasArea`.
    HasArea,
    /// `geo:hasMetricArea`.
    HasMetricArea,
    /// `geo:hasVolume`.
    HasVolume,
    /// `geo:hasMetricVolume`.
    HasMetricVolume,
}

impl GeoTerm {
    /// Every term, in a fixed order.
    pub const ALL: [Self; 41] = [
        Self::WktLiteral,
        Self::GmlLiteral,
        Self::GeoJsonLiteral,
        Self::KmlLiteral,
        Self::DggsLiteral,
        Self::SpatialObject,
        Self::Feature,
        Self::Geometry,
        Self::SpatialObjectCollection,
        Self::FeatureCollection,
        Self::GeometryCollection,
        Self::HasGeometry,
        Self::HasDefaultGeometry,
        Self::DefaultGeometry,
        Self::HasBoundingBox,
        Self::HasCentroid,
        Self::HasSerialization,
        Self::AsWkt,
        Self::AsGml,
        Self::AsGeoJson,
        Self::AsKml,
        Self::AsDggs,
        Self::Dimension,
        Self::CoordinateDimension,
        Self::SpatialDimension,
        Self::IsEmpty,
        Self::IsSimple,
        Self::HasSpatialResolution,
        Self::HasMetricSpatialResolution,
        Self::HasSpatialAccuracy,
        Self::HasMetricSpatialAccuracy,
        Self::HasSize,
        Self::HasMetricSize,
        Self::HasLength,
        Self::HasMetricLength,
        Self::HasPerimeterLength,
        Self::HasMetricPerimeterLength,
        Self::HasArea,
        Self::HasMetricArea,
        Self::HasVolume,
        Self::HasMetricVolume,
    ];

    /// The term's local name within the `geo:` namespace, byte-exact.
    #[must_use]
    pub const fn local_name(self) -> &'static str {
        match self {
            Self::WktLiteral => "wktLiteral",
            Self::GmlLiteral => "gmlLiteral",
            Self::GeoJsonLiteral => "geoJSONLiteral",
            Self::KmlLiteral => "kmlLiteral",
            Self::DggsLiteral => "dggsLiteral",
            Self::SpatialObject => "SpatialObject",
            Self::Feature => "Feature",
            Self::Geometry => "Geometry",
            Self::SpatialObjectCollection => "SpatialObjectCollection",
            Self::FeatureCollection => "FeatureCollection",
            Self::GeometryCollection => "GeometryCollection",
            Self::HasGeometry => "hasGeometry",
            Self::HasDefaultGeometry => "hasDefaultGeometry",
            Self::DefaultGeometry => "defaultGeometry",
            Self::HasBoundingBox => "hasBoundingBox",
            Self::HasCentroid => "hasCentroid",
            Self::HasSerialization => "hasSerialization",
            Self::AsWkt => "asWKT",
            Self::AsGml => "asGML",
            Self::AsGeoJson => "asGeoJSON",
            Self::AsKml => "asKML",
            Self::AsDggs => "asDGGS",
            Self::Dimension => "dimension",
            Self::CoordinateDimension => "coordinateDimension",
            Self::SpatialDimension => "spatialDimension",
            Self::IsEmpty => "isEmpty",
            Self::IsSimple => "isSimple",
            Self::HasSpatialResolution => "hasSpatialResolution",
            Self::HasMetricSpatialResolution => "hasMetricSpatialResolution",
            Self::HasSpatialAccuracy => "hasSpatialAccuracy",
            Self::HasMetricSpatialAccuracy => "hasMetricSpatialAccuracy",
            Self::HasSize => "hasSize",
            Self::HasMetricSize => "hasMetricSize",
            Self::HasLength => "hasLength",
            Self::HasMetricLength => "hasMetricLength",
            Self::HasPerimeterLength => "hasPerimeterLength",
            Self::HasMetricPerimeterLength => "hasMetricPerimeterLength",
            Self::HasArea => "hasArea",
            Self::HasMetricArea => "hasMetricArea",
            Self::HasVolume => "hasVolume",
            Self::HasMetricVolume => "hasMetricVolume",
        }
    }
}

/// The declared linear unit of one coordinate reference system.
///
/// A pair rather than two parallel lists, so a CRS and its unit cannot be
/// mis-aligned by a reader or by an edit.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CrsUnit {
    crs: String,
    unit: String,
}

impl CrsUnit {
    /// The coordinate reference system IRI.
    #[must_use]
    pub fn crs(&self) -> &str {
        &self.crs
    }

    /// The IRI of that system's linear unit.
    #[must_use]
    pub fn unit(&self) -> &str {
        &self.unit
    }
}

/// The GeoSPARQL vocabulary a host supplies.
///
/// Built with [`GeoVocabBuilder`]. There is deliberately no `Default`: see the
/// module docs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeoVocab {
    core_ns: String,
    function_ns: String,
    default_wkt_crs: Crs,
    geojson_crs: Crs,
    /// Sorted by CRS IRI at build time, so lookup and rendering are pure
    /// functions of the declared set rather than of the declaration order.
    crs_units: Vec<CrsUnit>,
    metre_unit: Option<String>,
    simple_features_ns: Option<String>,
    coordinate_scale: u32,
}

impl GeoVocab {
    /// The `geo:` namespace, byte-exact.
    #[must_use]
    pub fn core_namespace(&self) -> &str {
        &self.core_ns
    }

    /// The `geof:` namespace, byte-exact.
    #[must_use]
    pub fn function_namespace(&self) -> &str {
        &self.function_ns
    }

    /// The full IRI of a `geo:` term.
    #[must_use]
    pub fn term(&self, term: GeoTerm) -> String {
        format!("{}{}", self.core_ns, term.local_name())
    }

    /// The full IRI of a `geof:` function with the given local name.
    #[must_use]
    pub fn function(&self, local_name: &str) -> String {
        format!("{}{local_name}", self.function_ns)
    }

    /// The `geo:` term an IRI names, or `None` if it is not in the `geo:`
    /// namespace or names no term this crate knows.
    #[must_use]
    pub fn term_of(&self, iri: &str) -> Option<GeoTerm> {
        let local = iri.strip_prefix(self.core_ns.as_str())?;
        GeoTerm::ALL
            .into_iter()
            .find(|term| term.local_name() == local)
    }

    /// The coordinate reference system a `geo:wktLiteral` without an explicit
    /// `<IRI>` prefix is expressed in (OGC 22-047r1 Requirement 15).
    #[must_use]
    pub const fn default_wkt_crs(&self) -> &Crs {
        &self.default_wkt_crs
    }

    /// The coordinate reference system every `geo:geoJSONLiteral` is expressed in
    /// (Requirement 26 — RFC 7946 admits exactly one).
    #[must_use]
    pub const fn geojson_crs(&self) -> &Crs {
        &self.geojson_crs
    }

    /// The maximum number of fraction digits an emitted coordinate carries.
    ///
    /// A parameter rather than a constant because the fidelity a round trip needs
    /// is the caller's requirement, and because an exact coordinate can need more
    /// digits than any fixed choice would allow.
    #[must_use]
    pub const fn coordinate_scale(&self) -> u32 {
        self.coordinate_scale
    }

    /// Every declared coordinate-reference-system unit, sorted by CRS IRI.
    #[must_use]
    pub fn crs_units(&self) -> &[CrsUnit] {
        &self.crs_units
    }

    /// The declared linear unit of `crs`.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] naming `crs` when the caller has not declared its
    /// unit. This is the refusal the module docs describe: a measurement in an
    /// undeclared unit would be a plausible number that is silently wrong.
    pub fn unit_of(&self, crs: &Crs) -> Result<&str, GeoError> {
        self.crs_units
            .iter()
            .find(|entry| entry.crs == crs.as_str())
            .map(|entry| entry.unit.as_str())
            .ok_or_else(|| {
                GeoError::config(format!(
                    "no linear unit is declared for the coordinate reference system <{crs}>; a \
                     measurement cannot be reported in a unit purrdf-geo was never told, and \
                     PurRDF ships no coordinate-reference-system database to derive one from — \
                     declare it with GeoVocabBuilder::declare_crs_unit"
                ))
            })
    }

    /// Check that a measurement of a geometry in `crs` may be reported in
    /// `requested_unit`.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] when `crs` has no declared unit, and
    /// [`GeoError::Domain`] when the declared unit is a different IRI. This crate
    /// converts between units no more than it reprojects between systems: the
    /// planar measurement it computes is in the system's own unit, and reporting
    /// it under any other name would be a wrong number wearing a right label.
    pub fn require_unit(&self, crs: &Crs, requested_unit: &str) -> Result<(), GeoError> {
        let declared = self.unit_of(crs)?;
        if declared == requested_unit {
            return Ok(());
        }
        Err(GeoError::domain(format!(
            "a measurement of a geometry in <{crs}> is computed in that system's declared unit \
             <{declared}>, and <{requested_unit}> was requested; purrdf-geo converts no units, \
             so it refuses rather than reporting the number under a unit it does not denote"
        )))
    }

    /// Check that a `metric*` measurement of a geometry in `crs` is meaningful —
    /// that is, that the caller has declared the system's linear unit to be the
    /// metre.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] when no metre IRI or no unit for `crs` has been
    /// declared, and [`GeoError::Domain`] when the system's unit is not the
    /// metre.
    pub fn require_metre(&self, crs: &Crs) -> Result<(), GeoError> {
        let metre = self.metre_unit.as_deref().ok_or_else(|| {
            GeoError::config(
                "the metric GeoSPARQL functions report metres, and no IRI has been declared as \
                 the metre; PurRDF mints no vocabulary IRIs, so declare it with \
                 GeoVocabBuilder::declare_metre",
            )
        })?;
        self.require_unit(crs, metre)
    }

    /// The IRI the caller declared as the metre, if any.
    #[must_use]
    pub fn metre_unit(&self) -> Option<&str> {
        self.metre_unit.as_deref()
    }

    /// The IRI naming `kind` in the OGC Simple Features geometry-type
    /// vocabulary — the answer `geof:geometryType` returns.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] when the caller has not declared the Simple Features
    /// namespace. `geof:geometryType` answers with an IRI from a *third*
    /// vocabulary (`sf:`), which is no more PurRDF's to mint than `geo:` or
    /// `geof:` is, so the function is inert until the caller supplies it rather
    /// than inventing a namespace to answer from.
    pub fn simple_features_type(
        &self,
        kind: crate::geom::GeometryKind,
    ) -> Result<String, GeoError> {
        let namespace = self.simple_features_ns.as_deref().ok_or_else(|| {
            GeoError::config(
                "geof:geometryType answers with an IRI from the OGC Simple Features geometry-type \
                 vocabulary, and no namespace has been declared for it; PurRDF mints no \
                 vocabulary IRIs, so declare it with \
                 GeoVocabBuilder::declare_simple_features_namespace",
            )
        })?;
        // The `sf:` local names are the Simple Features class names, which are
        // the WKT keywords in upper-camel case. `GeometryKind::geojson_type`
        // already spells exactly those (`Point`, `LineString`, `MultiPolygon`,
        // `GeometryCollection`), because RFC 7946 took its type names from the
        // same source; reusing it keeps one spelling rather than two that could
        // drift.
        Ok(format!("{namespace}{}", kind.geojson_type()))
    }
}

/// Builder for [`GeoVocab`].
///
/// The four required values are constructor arguments rather than optional
/// setters, so a vocabulary cannot be built half-configured and discover the gap
/// at query time.
#[derive(Clone, Debug)]
pub struct GeoVocabBuilder {
    core_ns: String,
    function_ns: String,
    default_wkt_crs: Crs,
    geojson_crs: Crs,
    crs_units: Vec<CrsUnit>,
    metre_unit: Option<String>,
    simple_features_ns: Option<String>,
    coordinate_scale: u32,
}

/// The default maximum fraction digits an emitted coordinate carries.
///
/// Fifteen because that is the most a `xsd:double` round trip can preserve, so a
/// consumer that reads an emitted coordinate back through a double-based store
/// loses nothing this crate could have kept. It is a *default*, not a constant:
/// [`GeoVocabBuilder::coordinate_scale`] raises or lowers it, and an exact
/// coordinate that needs more digits is a legitimate thing to ask for.
pub const DEFAULT_COORDINATE_SCALE: u32 = 15;

impl GeoVocabBuilder {
    /// Begin a vocabulary from the two namespaces and the two coordinate
    /// reference systems the standard's own conformance classes need.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] if either namespace is empty. An empty namespace
    /// would make every term IRI a bare local name, which would collide with
    /// every other vocabulary in the dataset.
    pub fn new(
        core_namespace: impl Into<String>,
        function_namespace: impl Into<String>,
        default_wkt_crs: Crs,
        geojson_crs: Crs,
    ) -> Result<Self, GeoError> {
        let core_ns = core_namespace.into();
        let function_ns = function_namespace.into();
        for (label, value) in [("geo:", &core_ns), ("geof:", &function_ns)] {
            if value.is_empty() {
                return Err(GeoError::config(format!(
                    "the {label} namespace may not be empty; PurRDF mints no vocabulary IRIs, so \
                     the GeoSPARQL namespaces must be supplied by the caller"
                )));
            }
        }
        Ok(Self {
            core_ns,
            function_ns,
            default_wkt_crs,
            geojson_crs,
            crs_units: Vec::new(),
            metre_unit: None,
            simple_features_ns: None,
            coordinate_scale: DEFAULT_COORDINATE_SCALE,
        })
    }

    /// Declare the linear unit of one coordinate reference system.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] if `unit` is empty, or if `crs` already has a
    /// declared unit that differs. A silently overwritten declaration would
    /// change every measurement's meaning with no symptom; a repeat of the same
    /// declaration is harmless and is accepted.
    pub fn declare_crs_unit(
        mut self,
        crs: &Crs,
        unit: impl Into<String>,
    ) -> Result<Self, GeoError> {
        let unit = unit.into();
        if unit.is_empty() {
            return Err(GeoError::config(
                "a unit IRI may not be empty; PurRDF mints no vocabulary IRIs, so the unit of a \
                 coordinate reference system must be supplied by the caller",
            ));
        }
        if let Some(existing) = self
            .crs_units
            .iter()
            .find(|entry| entry.crs == crs.as_str())
        {
            if existing.unit == unit {
                return Ok(self);
            }
            return Err(GeoError::config(format!(
                "<{crs}> is already declared to have the linear unit <{}>, and <{unit}> was \
                 declared for it as well; a silently replaced declaration would change every \
                 measurement's meaning with no symptom",
                existing.unit
            )));
        }
        self.crs_units.push(CrsUnit {
            crs: crs.as_str().to_owned(),
            unit,
        });
        Ok(self)
    }

    /// Declare which unit IRI denotes the metre, enabling the `metric*` family
    /// for every coordinate reference system whose declared unit is that IRI.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] if `iri` is empty.
    pub fn declare_metre(mut self, iri: impl Into<String>) -> Result<Self, GeoError> {
        let iri = iri.into();
        if iri.is_empty() {
            return Err(GeoError::config("the metre IRI may not be empty"));
        }
        self.metre_unit = Some(iri);
        Ok(self)
    }

    /// Declare the OGC Simple Features geometry-type namespace, enabling
    /// `geof:geometryType`.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] if `iri` is empty.
    pub fn declare_simple_features_namespace(
        mut self,
        iri: impl Into<String>,
    ) -> Result<Self, GeoError> {
        let iri = iri.into();
        if iri.is_empty() {
            return Err(GeoError::config(
                "the Simple Features geometry-type namespace may not be empty",
            ));
        }
        self.simple_features_ns = Some(iri);
        Ok(self)
    }

    /// Set the maximum number of fraction digits an emitted coordinate carries.
    #[must_use]
    pub fn coordinate_scale(mut self, digits: u32) -> Self {
        self.coordinate_scale = digits;
        self
    }

    /// Finish the vocabulary.
    #[must_use]
    pub fn build(mut self) -> GeoVocab {
        // Sorted so that two hosts declaring the same units in different orders
        // build equal vocabularies, and so `Debug` and `crs_units` are pure
        // functions of the declared set.
        self.crs_units.sort_by(|a, b| a.crs.cmp(&b.crs));
        GeoVocab {
            core_ns: self.core_ns,
            function_ns: self.function_ns,
            default_wkt_crs: self.default_wkt_crs,
            geojson_crs: self.geojson_crs,
            crs_units: self.crs_units,
            metre_unit: self.metre_unit,
            simple_features_ns: self.simple_features_ns,
            coordinate_scale: self.coordinate_scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_COORDINATE_SCALE, GeoTerm, GeoVocab, GeoVocabBuilder};
    use crate::error::GeoError;
    use crate::geom::Crs;

    const CORE: &str = "http://example.org/geo#";
    const FUNC: &str = "http://example.org/geof/";
    const CRS_A: &str = "http://example.org/crs/A";
    const CRS_B: &str = "http://example.org/crs/B";
    const METRE: &str = "http://example.org/unit/metre";
    const DEGREE: &str = "http://example.org/unit/degree";

    fn crs(iri: &str) -> Crs {
        Crs::new(iri).expect("a non-empty IRI")
    }

    fn vocab() -> GeoVocab {
        GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A))
            .expect("non-empty namespaces")
            .declare_crs_unit(&crs(CRS_A), METRE)
            .expect("a fresh declaration")
            .declare_metre(METRE)
            .expect("a non-empty metre IRI")
            .build()
    }

    /// The type this crate would need in order to mint a vocabulary of its own
    /// deliberately has no `Default`. This test cannot assert the absence of a
    /// trait impl, so it asserts the property that absence protects: there is no
    /// constructor that does not take the namespaces.
    #[test]
    fn a_vocabulary_cannot_be_built_without_the_callers_namespaces() {
        assert!(matches!(
            GeoVocabBuilder::new("", FUNC, crs(CRS_A), crs(CRS_A)),
            Err(GeoError::Config(_))
        ));
        assert!(matches!(
            GeoVocabBuilder::new(CORE, "", crs(CRS_A), crs(CRS_A)),
            Err(GeoError::Config(_))
        ));
        // The neighbouring VALID case.
        assert!(GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A)).is_ok());
    }

    #[test]
    fn every_term_has_a_distinct_local_name_and_round_trips_through_its_iri() {
        let vocab = vocab();
        let mut names: Vec<&str> = GeoTerm::ALL.iter().map(|t| t.local_name()).collect();
        assert_eq!(
            names.len(),
            41,
            "the ALL table must list every variant once"
        );
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "no two terms share a local name");

        for term in GeoTerm::ALL {
            let iri = vocab.term(term);
            assert!(
                iri.starts_with(CORE),
                "{iri} must sit in the caller's namespace"
            );
            assert_eq!(
                vocab.term_of(&iri),
                Some(term),
                "{iri} must resolve back to its term"
            );
        }
        assert_eq!(
            vocab.term_of("http://example.org/geo#notATerm"),
            None,
            "an unknown local name resolves to nothing rather than to a default"
        );
        assert_eq!(
            vocab.term_of("http://elsewhere.example.org/geo#wktLiteral"),
            None,
            "a term outside the caller's namespace is not this vocabulary's"
        );
    }

    /// The two local names the standard's own summary table gets wrong or that a
    /// reader is most likely to guess wrongly.
    #[test]
    fn the_easily_mistyped_local_names_are_the_ontologys() {
        let vocab = vocab();
        assert_eq!(
            vocab.term(GeoTerm::GeoJsonLiteral),
            format!("{CORE}geoJSONLiteral")
        );
        assert_eq!(vocab.term(GeoTerm::AsWkt), format!("{CORE}asWKT"));
        assert_eq!(
            vocab.term(GeoTerm::HasPerimeterLength),
            format!("{CORE}hasPerimeterLength"),
            "there is no geo:hasPerimeter, even though the function is geof:perimeter"
        );
        assert_eq!(
            vocab.term(GeoTerm::HasDefaultGeometry),
            format!("{CORE}hasDefaultGeometry")
        );
    }

    #[test]
    fn the_function_namespace_is_applied_verbatim() {
        let vocab = vocab();
        assert_eq!(vocab.function("sfWithin"), format!("{FUNC}sfWithin"));
        assert_eq!(vocab.function_namespace(), FUNC);
    }

    // ---- units -----------------------------------------------------------

    #[test]
    fn an_undeclared_crs_unit_is_refused_and_a_declared_one_is_not() {
        let vocab = vocab();
        assert!(
            matches!(vocab.unit_of(&crs(CRS_B)), Err(GeoError::Config(_))),
            "a system with no declared unit cannot be measured in one"
        );
        // The neighbouring VALID case.
        assert_eq!(vocab.unit_of(&crs(CRS_A)).expect("declared"), METRE);
    }

    #[test]
    fn a_measurement_in_the_wrong_unit_is_refused_and_the_right_one_is_not() {
        let vocab = vocab();
        assert!(
            matches!(
                vocab.require_unit(&crs(CRS_A), DEGREE),
                Err(GeoError::Domain(_))
            ),
            "this crate converts no units"
        );
        // The neighbouring VALID case.
        assert!(vocab.require_unit(&crs(CRS_A), METRE).is_ok());
    }

    #[test]
    fn the_metric_family_needs_both_a_metre_iri_and_a_metre_crs() {
        // No metre declared at all.
        let no_metre = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A))
            .expect("namespaces")
            .declare_crs_unit(&crs(CRS_A), METRE)
            .expect("declaration")
            .build();
        assert!(
            matches!(
                no_metre.require_metre(&crs(CRS_A)),
                Err(GeoError::Config(_))
            ),
            "without a declared metre IRI the metric family cannot answer"
        );

        // A metre IRI, but a system measured in degrees.
        let degrees = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A))
            .expect("namespaces")
            .declare_crs_unit(&crs(CRS_B), DEGREE)
            .expect("declaration")
            .declare_metre(METRE)
            .expect("metre")
            .build();
        assert!(
            matches!(degrees.require_metre(&crs(CRS_B)), Err(GeoError::Domain(_))),
            "a planar measurement in degrees is not a measurement in metres"
        );

        // The neighbouring VALID case: a metre IRI and a system measured in metres.
        assert!(vocab().require_metre(&crs(CRS_A)).is_ok());
    }

    #[test]
    fn a_repeated_identical_declaration_is_accepted_and_a_conflicting_one_is_not() {
        let builder = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A))
            .expect("namespaces")
            .declare_crs_unit(&crs(CRS_A), METRE)
            .expect("first");
        // The same declaration again is harmless.
        let builder = builder
            .declare_crs_unit(&crs(CRS_A), METRE)
            .expect("a repeat of the same declaration is not a conflict");
        // A different one for the same system is a hard error.
        assert!(matches!(
            builder.declare_crs_unit(&crs(CRS_A), DEGREE),
            Err(GeoError::Config(_))
        ));
    }

    #[test]
    fn declarations_are_sorted_so_declaration_order_cannot_reach_a_result() {
        let forward = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A))
            .expect("namespaces")
            .declare_crs_unit(&crs(CRS_B), DEGREE)
            .expect("b")
            .declare_crs_unit(&crs(CRS_A), METRE)
            .expect("a")
            .build();
        let backward = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A))
            .expect("namespaces")
            .declare_crs_unit(&crs(CRS_A), METRE)
            .expect("a")
            .declare_crs_unit(&crs(CRS_B), DEGREE)
            .expect("b")
            .build();
        assert_eq!(
            forward, backward,
            "two hosts declaring the same units in different orders build equal vocabularies"
        );
        assert_eq!(forward.crs_units()[0].crs(), CRS_A);
    }

    #[test]
    fn the_coordinate_scale_defaults_and_is_settable() {
        assert_eq!(vocab().coordinate_scale(), DEFAULT_COORDINATE_SCALE);
        let wide = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A))
            .expect("namespaces")
            .coordinate_scale(40)
            .build();
        assert_eq!(wide.coordinate_scale(), 40);
    }

    /// `geof:geometryType` answers from a third vocabulary, and is inert until
    /// the caller supplies it rather than inventing a namespace.
    #[test]
    fn geometry_type_is_inert_until_the_simple_features_namespace_is_declared() {
        use crate::geom::GeometryKind;
        assert!(matches!(
            vocab().simple_features_type(GeometryKind::Polygon),
            Err(GeoError::Config(_))
        ));
        // The neighbouring VALID case.
        let declared = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_A))
            .expect("namespaces")
            .declare_simple_features_namespace("http://example.org/sf#")
            .expect("a non-empty namespace")
            .build();
        assert_eq!(
            declared
                .simple_features_type(GeometryKind::MultiPolygon)
                .expect("declared"),
            "http://example.org/sf#MultiPolygon"
        );
        assert_eq!(
            declared
                .simple_features_type(GeometryKind::GeometryCollection)
                .expect("declared"),
            "http://example.org/sf#GeometryCollection"
        );
    }

    #[test]
    fn the_two_coordinate_reference_systems_are_carried_verbatim() {
        let vocab = GeoVocabBuilder::new(CORE, FUNC, crs(CRS_A), crs(CRS_B))
            .expect("namespaces")
            .build();
        assert_eq!(vocab.default_wkt_crs().as_str(), CRS_A);
        assert_eq!(vocab.geojson_crs().as_str(), CRS_B);
    }
}
