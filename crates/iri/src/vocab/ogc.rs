// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Official OGC GeoSPARQL 1.1 terms and carrier references.
//!
//! Source: <https://docs.ogc.org/is/22-047r1/22-047r1.html>.
//! These standard declarations supply the immutable geographic default;
//! application vocabularies and every other reference remain caller-supplied.

/// WGS84 longitude/latitude, the GeoSPARQL default WKT and GeoJSON reference.
pub const CRS84: &str = "http://www.opengis.net/def/crs/OGC/1.3/CRS84";

/// EPSG:4326; callers must explicitly register its latitude/longitude axes.
pub const EPSG4326: &str = "http://www.opengis.net/def/crs/EPSG/0/4326";

/// The OGC `geo:` terms defined by GeoSPARQL 1.1.
pub mod geo {
    /// The official namespace IRI.
    pub const NS: &str = "http://www.opengis.net/ont/geosparql#";

    /// `geo:wktLiteral`.
    pub const WKT_LITERAL: &str = "http://www.opengis.net/ont/geosparql#wktLiteral";

    /// `geo:gmlLiteral`.
    pub const GML_LITERAL: &str = "http://www.opengis.net/ont/geosparql#gmlLiteral";

    /// `geo:geoJSONLiteral`.
    pub const GEO_JSON_LITERAL: &str = "http://www.opengis.net/ont/geosparql#geoJSONLiteral";

    /// `geo:kmlLiteral`.
    pub const KML_LITERAL: &str = "http://www.opengis.net/ont/geosparql#kmlLiteral";

    /// `geo:dggsLiteral`.
    pub const DGGS_LITERAL: &str = "http://www.opengis.net/ont/geosparql#dggsLiteral";

    /// `geo:SpatialObject`.
    pub const SPATIAL_OBJECT: &str = "http://www.opengis.net/ont/geosparql#SpatialObject";

    /// `geo:Feature`.
    pub const FEATURE: &str = "http://www.opengis.net/ont/geosparql#Feature";

    /// `geo:Geometry`.
    pub const GEOMETRY: &str = "http://www.opengis.net/ont/geosparql#Geometry";

    /// `geo:SpatialObjectCollection`.
    pub const SPATIAL_OBJECT_COLLECTION: &str =
        "http://www.opengis.net/ont/geosparql#SpatialObjectCollection";

    /// `geo:FeatureCollection`.
    pub const FEATURE_COLLECTION: &str = "http://www.opengis.net/ont/geosparql#FeatureCollection";

    /// `geo:GeometryCollection`.
    pub const GEOMETRY_COLLECTION: &str = "http://www.opengis.net/ont/geosparql#GeometryCollection";

    /// `geo:hasGeometry`.
    pub const HAS_GEOMETRY: &str = "http://www.opengis.net/ont/geosparql#hasGeometry";

    /// `geo:hasDefaultGeometry`.
    pub const HAS_DEFAULT_GEOMETRY: &str =
        "http://www.opengis.net/ont/geosparql#hasDefaultGeometry";

    /// `geo:defaultGeometry`.
    pub const DEFAULT_GEOMETRY: &str = "http://www.opengis.net/ont/geosparql#defaultGeometry";

    /// `geo:hasBoundingBox`.
    pub const HAS_BOUNDING_BOX: &str = "http://www.opengis.net/ont/geosparql#hasBoundingBox";

    /// `geo:hasCentroid`.
    pub const HAS_CENTROID: &str = "http://www.opengis.net/ont/geosparql#hasCentroid";

    /// `geo:hasSerialization`.
    pub const HAS_SERIALIZATION: &str = "http://www.opengis.net/ont/geosparql#hasSerialization";

    /// `geo:asWKT`.
    pub const AS_WKT: &str = "http://www.opengis.net/ont/geosparql#asWKT";

    /// `geo:asGML`.
    pub const AS_GML: &str = "http://www.opengis.net/ont/geosparql#asGML";

    /// `geo:asGeoJSON`.
    pub const AS_GEO_JSON: &str = "http://www.opengis.net/ont/geosparql#asGeoJSON";

    /// `geo:asKML`.
    pub const AS_KML: &str = "http://www.opengis.net/ont/geosparql#asKML";

    /// `geo:asDGGS`.
    pub const AS_DGGS: &str = "http://www.opengis.net/ont/geosparql#asDGGS";

    /// `geo:dimension`.
    pub const DIMENSION: &str = "http://www.opengis.net/ont/geosparql#dimension";

    /// `geo:coordinateDimension`.
    pub const COORDINATE_DIMENSION: &str =
        "http://www.opengis.net/ont/geosparql#coordinateDimension";

    /// `geo:spatialDimension`.
    pub const SPATIAL_DIMENSION: &str = "http://www.opengis.net/ont/geosparql#spatialDimension";

    /// `geo:isEmpty`.
    pub const IS_EMPTY: &str = "http://www.opengis.net/ont/geosparql#isEmpty";

    /// `geo:isSimple`.
    pub const IS_SIMPLE: &str = "http://www.opengis.net/ont/geosparql#isSimple";

    /// `geo:hasSpatialResolution`.
    pub const HAS_SPATIAL_RESOLUTION: &str =
        "http://www.opengis.net/ont/geosparql#hasSpatialResolution";

    /// `geo:hasMetricSpatialResolution`.
    pub const HAS_METRIC_SPATIAL_RESOLUTION: &str =
        "http://www.opengis.net/ont/geosparql#hasMetricSpatialResolution";

    /// `geo:hasSpatialAccuracy`.
    pub const HAS_SPATIAL_ACCURACY: &str =
        "http://www.opengis.net/ont/geosparql#hasSpatialAccuracy";

    /// `geo:hasMetricSpatialAccuracy`.
    pub const HAS_METRIC_SPATIAL_ACCURACY: &str =
        "http://www.opengis.net/ont/geosparql#hasMetricSpatialAccuracy";

    /// `geo:hasSize`.
    pub const HAS_SIZE: &str = "http://www.opengis.net/ont/geosparql#hasSize";

    /// `geo:hasMetricSize`.
    pub const HAS_METRIC_SIZE: &str = "http://www.opengis.net/ont/geosparql#hasMetricSize";

    /// `geo:hasLength`.
    pub const HAS_LENGTH: &str = "http://www.opengis.net/ont/geosparql#hasLength";

    /// `geo:hasMetricLength`.
    pub const HAS_METRIC_LENGTH: &str = "http://www.opengis.net/ont/geosparql#hasMetricLength";

    /// `geo:hasPerimeterLength`.
    pub const HAS_PERIMETER_LENGTH: &str =
        "http://www.opengis.net/ont/geosparql#hasPerimeterLength";

    /// `geo:hasMetricPerimeterLength`.
    pub const HAS_METRIC_PERIMETER_LENGTH: &str =
        "http://www.opengis.net/ont/geosparql#hasMetricPerimeterLength";

    /// `geo:hasArea`.
    pub const HAS_AREA: &str = "http://www.opengis.net/ont/geosparql#hasArea";

    /// `geo:hasMetricArea`.
    pub const HAS_METRIC_AREA: &str = "http://www.opengis.net/ont/geosparql#hasMetricArea";

    /// `geo:hasVolume`.
    pub const HAS_VOLUME: &str = "http://www.opengis.net/ont/geosparql#hasVolume";

    /// `geo:hasMetricVolume`.
    pub const HAS_METRIC_VOLUME: &str = "http://www.opengis.net/ont/geosparql#hasMetricVolume";

    /// `geo:sfEquals`.
    pub const SF_EQUALS: &str = "http://www.opengis.net/ont/geosparql#sfEquals";

    /// `geo:sfDisjoint`.
    pub const SF_DISJOINT: &str = "http://www.opengis.net/ont/geosparql#sfDisjoint";

    /// `geo:sfIntersects`.
    pub const SF_INTERSECTS: &str = "http://www.opengis.net/ont/geosparql#sfIntersects";

    /// `geo:sfTouches`.
    pub const SF_TOUCHES: &str = "http://www.opengis.net/ont/geosparql#sfTouches";

    /// `geo:sfCrosses`.
    pub const SF_CROSSES: &str = "http://www.opengis.net/ont/geosparql#sfCrosses";

    /// `geo:sfWithin`.
    pub const SF_WITHIN: &str = "http://www.opengis.net/ont/geosparql#sfWithin";

    /// `geo:sfContains`.
    pub const SF_CONTAINS: &str = "http://www.opengis.net/ont/geosparql#sfContains";

    /// `geo:sfOverlaps`.
    pub const SF_OVERLAPS: &str = "http://www.opengis.net/ont/geosparql#sfOverlaps";

    /// `geo:ehEquals`.
    pub const EH_EQUALS: &str = "http://www.opengis.net/ont/geosparql#ehEquals";

    /// `geo:ehDisjoint`.
    pub const EH_DISJOINT: &str = "http://www.opengis.net/ont/geosparql#ehDisjoint";

    /// `geo:ehMeet`.
    pub const EH_MEET: &str = "http://www.opengis.net/ont/geosparql#ehMeet";

    /// `geo:ehOverlap`.
    pub const EH_OVERLAP: &str = "http://www.opengis.net/ont/geosparql#ehOverlap";

    /// `geo:ehCovers`.
    pub const EH_COVERS: &str = "http://www.opengis.net/ont/geosparql#ehCovers";

    /// `geo:ehCoveredBy`.
    pub const EH_COVERED_BY: &str = "http://www.opengis.net/ont/geosparql#ehCoveredBy";

    /// `geo:ehInside`.
    pub const EH_INSIDE: &str = "http://www.opengis.net/ont/geosparql#ehInside";

    /// `geo:ehContains`.
    pub const EH_CONTAINS: &str = "http://www.opengis.net/ont/geosparql#ehContains";

    /// `geo:rcc8eq`.
    pub const RCC8EQ: &str = "http://www.opengis.net/ont/geosparql#rcc8eq";

    /// `geo:rcc8dc`.
    pub const RCC8DC: &str = "http://www.opengis.net/ont/geosparql#rcc8dc";

    /// `geo:rcc8ec`.
    pub const RCC8EC: &str = "http://www.opengis.net/ont/geosparql#rcc8ec";

    /// `geo:rcc8po`.
    pub const RCC8PO: &str = "http://www.opengis.net/ont/geosparql#rcc8po";

    /// `geo:rcc8tppi`.
    pub const RCC8TPPI: &str = "http://www.opengis.net/ont/geosparql#rcc8tppi";

    /// `geo:rcc8tpp`.
    pub const RCC8TPP: &str = "http://www.opengis.net/ont/geosparql#rcc8tpp";

    /// `geo:rcc8ntpp`.
    pub const RCC8NTPP: &str = "http://www.opengis.net/ont/geosparql#rcc8ntpp";

    /// `geo:rcc8ntppi`.
    pub const RCC8NTPPI: &str = "http://www.opengis.net/ont/geosparql#rcc8ntppi";
}

/// The OGC `geof:` terms defined by GeoSPARQL 1.1.
pub mod geof {
    /// The official namespace IRI.
    pub const NS: &str = "http://www.opengis.net/def/function/geosparql/";

    /// `geof:sfEquals`.
    pub const SF_EQUALS: &str = "http://www.opengis.net/def/function/geosparql/sfEquals";

    /// `geof:sfDisjoint`.
    pub const SF_DISJOINT: &str = "http://www.opengis.net/def/function/geosparql/sfDisjoint";

    /// `geof:sfIntersects`.
    pub const SF_INTERSECTS: &str = "http://www.opengis.net/def/function/geosparql/sfIntersects";

    /// `geof:sfTouches`.
    pub const SF_TOUCHES: &str = "http://www.opengis.net/def/function/geosparql/sfTouches";

    /// `geof:sfCrosses`.
    pub const SF_CROSSES: &str = "http://www.opengis.net/def/function/geosparql/sfCrosses";

    /// `geof:sfWithin`.
    pub const SF_WITHIN: &str = "http://www.opengis.net/def/function/geosparql/sfWithin";

    /// `geof:sfContains`.
    pub const SF_CONTAINS: &str = "http://www.opengis.net/def/function/geosparql/sfContains";

    /// `geof:sfOverlaps`.
    pub const SF_OVERLAPS: &str = "http://www.opengis.net/def/function/geosparql/sfOverlaps";

    /// `geof:ehEquals`.
    pub const EH_EQUALS: &str = "http://www.opengis.net/def/function/geosparql/ehEquals";

    /// `geof:ehDisjoint`.
    pub const EH_DISJOINT: &str = "http://www.opengis.net/def/function/geosparql/ehDisjoint";

    /// `geof:ehMeet`.
    pub const EH_MEET: &str = "http://www.opengis.net/def/function/geosparql/ehMeet";

    /// `geof:ehOverlap`.
    pub const EH_OVERLAP: &str = "http://www.opengis.net/def/function/geosparql/ehOverlap";

    /// `geof:ehCovers`.
    pub const EH_COVERS: &str = "http://www.opengis.net/def/function/geosparql/ehCovers";

    /// `geof:ehCoveredBy`.
    pub const EH_COVERED_BY: &str = "http://www.opengis.net/def/function/geosparql/ehCoveredBy";

    /// `geof:ehInside`.
    pub const EH_INSIDE: &str = "http://www.opengis.net/def/function/geosparql/ehInside";

    /// `geof:ehContains`.
    pub const EH_CONTAINS: &str = "http://www.opengis.net/def/function/geosparql/ehContains";

    /// `geof:rcc8eq`.
    pub const RCC8EQ: &str = "http://www.opengis.net/def/function/geosparql/rcc8eq";

    /// `geof:rcc8dc`.
    pub const RCC8DC: &str = "http://www.opengis.net/def/function/geosparql/rcc8dc";

    /// `geof:rcc8ec`.
    pub const RCC8EC: &str = "http://www.opengis.net/def/function/geosparql/rcc8ec";

    /// `geof:rcc8po`.
    pub const RCC8PO: &str = "http://www.opengis.net/def/function/geosparql/rcc8po";

    /// `geof:rcc8tppi`.
    pub const RCC8TPPI: &str = "http://www.opengis.net/def/function/geosparql/rcc8tppi";

    /// `geof:rcc8tpp`.
    pub const RCC8TPP: &str = "http://www.opengis.net/def/function/geosparql/rcc8tpp";

    /// `geof:rcc8ntpp`.
    pub const RCC8NTPP: &str = "http://www.opengis.net/def/function/geosparql/rcc8ntpp";

    /// `geof:rcc8ntppi`.
    pub const RCC8NTPPI: &str = "http://www.opengis.net/def/function/geosparql/rcc8ntppi";

    /// `geof:relate`.
    pub const RELATE: &str = "http://www.opengis.net/def/function/geosparql/relate";

    /// `geof:dimension`.
    pub const DIMENSION: &str = "http://www.opengis.net/def/function/geosparql/dimension";

    /// `geof:coordinateDimension`.
    pub const COORDINATE_DIMENSION: &str =
        "http://www.opengis.net/def/function/geosparql/coordinateDimension";

    /// `geof:spatialDimension`.
    pub const SPATIAL_DIMENSION: &str =
        "http://www.opengis.net/def/function/geosparql/spatialDimension";

    /// `geof:geometryType`.
    pub const GEOMETRY_TYPE: &str = "http://www.opengis.net/def/function/geosparql/geometryType";

    /// `geof:isEmpty`.
    pub const IS_EMPTY: &str = "http://www.opengis.net/def/function/geosparql/isEmpty";

    /// `geof:isSimple`.
    pub const IS_SIMPLE: &str = "http://www.opengis.net/def/function/geosparql/isSimple";

    /// `geof:is3D`.
    pub const IS3_D: &str = "http://www.opengis.net/def/function/geosparql/is3D";

    /// `geof:isMeasured`.
    pub const IS_MEASURED: &str = "http://www.opengis.net/def/function/geosparql/isMeasured";

    /// `geof:getSRID`.
    pub const GET_SRID: &str = "http://www.opengis.net/def/function/geosparql/getSRID";

    /// `geof:numGeometries`.
    pub const NUM_GEOMETRIES: &str = "http://www.opengis.net/def/function/geosparql/numGeometries";

    /// `geof:geometryN`.
    pub const GEOMETRY_N: &str = "http://www.opengis.net/def/function/geosparql/geometryN";

    /// `geof:minX`.
    pub const MIN_X: &str = "http://www.opengis.net/def/function/geosparql/minX";

    /// `geof:maxX`.
    pub const MAX_X: &str = "http://www.opengis.net/def/function/geosparql/maxX";

    /// `geof:minY`.
    pub const MIN_Y: &str = "http://www.opengis.net/def/function/geosparql/minY";

    /// `geof:maxY`.
    pub const MAX_Y: &str = "http://www.opengis.net/def/function/geosparql/maxY";

    /// `geof:minZ`.
    pub const MIN_Z: &str = "http://www.opengis.net/def/function/geosparql/minZ";

    /// `geof:maxZ`.
    pub const MAX_Z: &str = "http://www.opengis.net/def/function/geosparql/maxZ";

    /// `geof:envelope`.
    pub const ENVELOPE: &str = "http://www.opengis.net/def/function/geosparql/envelope";

    /// `geof:boundary`.
    pub const BOUNDARY: &str = "http://www.opengis.net/def/function/geosparql/boundary";

    /// `geof:convexHull`.
    pub const CONVEX_HULL: &str = "http://www.opengis.net/def/function/geosparql/convexHull";

    /// `geof:centroid`.
    pub const CENTROID: &str = "http://www.opengis.net/def/function/geosparql/centroid";

    /// `geof:area`.
    pub const AREA: &str = "http://www.opengis.net/def/function/geosparql/area";

    /// `geof:metricArea`.
    pub const METRIC_AREA: &str = "http://www.opengis.net/def/function/geosparql/metricArea";

    /// `geof:length`.
    pub const LENGTH: &str = "http://www.opengis.net/def/function/geosparql/length";

    /// `geof:metricLength`.
    pub const METRIC_LENGTH: &str = "http://www.opengis.net/def/function/geosparql/metricLength";

    /// `geof:perimeter`.
    pub const PERIMETER: &str = "http://www.opengis.net/def/function/geosparql/perimeter";

    /// `geof:metricPerimeter`.
    pub const METRIC_PERIMETER: &str =
        "http://www.opengis.net/def/function/geosparql/metricPerimeter";

    /// `geof:distance`.
    pub const DISTANCE: &str = "http://www.opengis.net/def/function/geosparql/distance";

    /// `geof:metricDistance`.
    pub const METRIC_DISTANCE: &str =
        "http://www.opengis.net/def/function/geosparql/metricDistance";

    /// `geof:asWKT`.
    pub const AS_WKT: &str = "http://www.opengis.net/def/function/geosparql/asWKT";

    /// `geof:asGeoJSON`.
    pub const AS_GEO_JSON: &str = "http://www.opengis.net/def/function/geosparql/asGeoJSON";

    /// `geof:transform`.
    pub const TRANSFORM: &str = "http://www.opengis.net/def/function/geosparql/transform";

    /// `geof:buffer`.
    pub const BUFFER: &str = "http://www.opengis.net/def/function/geosparql/buffer";

    /// `geof:metricBuffer`.
    pub const METRIC_BUFFER: &str = "http://www.opengis.net/def/function/geosparql/metricBuffer";

    /// `geof:boundingCircle`.
    pub const BOUNDING_CIRCLE: &str =
        "http://www.opengis.net/def/function/geosparql/boundingCircle";

    /// `geof:concaveHull`.
    pub const CONCAVE_HULL: &str = "http://www.opengis.net/def/function/geosparql/concaveHull";

    /// `geof:intersection`.
    pub const INTERSECTION: &str = "http://www.opengis.net/def/function/geosparql/intersection";

    /// `geof:union`.
    pub const UNION: &str = "http://www.opengis.net/def/function/geosparql/union";

    /// `geof:difference`.
    pub const DIFFERENCE: &str = "http://www.opengis.net/def/function/geosparql/difference";

    /// `geof:symDifference`.
    pub const SYM_DIFFERENCE: &str = "http://www.opengis.net/def/function/geosparql/symDifference";

    /// `geof:asGML`.
    pub const AS_GML: &str = "http://www.opengis.net/def/function/geosparql/asGML";

    /// `geof:asKML`.
    pub const AS_KML: &str = "http://www.opengis.net/def/function/geosparql/asKML";

    /// `geof:asDGGS`.
    pub const AS_DGGS: &str = "http://www.opengis.net/def/function/geosparql/asDGGS";

    /// `geof:aggBoundingBox`.
    pub const AGG_BOUNDING_BOX: &str =
        "http://www.opengis.net/def/function/geosparql/aggBoundingBox";

    /// `geof:aggBoundingCircle`.
    pub const AGG_BOUNDING_CIRCLE: &str =
        "http://www.opengis.net/def/function/geosparql/aggBoundingCircle";

    /// `geof:aggCentroid`.
    pub const AGG_CENTROID: &str = "http://www.opengis.net/def/function/geosparql/aggCentroid";

    /// `geof:aggConcaveHull`.
    pub const AGG_CONCAVE_HULL: &str =
        "http://www.opengis.net/def/function/geosparql/aggConcaveHull";

    /// `geof:aggConvexHull`.
    pub const AGG_CONVEX_HULL: &str = "http://www.opengis.net/def/function/geosparql/aggConvexHull";

    /// `geof:aggUnion`.
    pub const AGG_UNION: &str = "http://www.opengis.net/def/function/geosparql/aggUnion";
}

/// The OGC `sf:` terms defined by GeoSPARQL 1.1.
pub mod sf {
    /// The official namespace IRI.
    pub const NS: &str = "http://www.opengis.net/ont/sf#";

    /// `sf:Point`.
    pub const POINT: &str = "http://www.opengis.net/ont/sf#Point";

    /// `sf:LineString`.
    pub const LINE_STRING: &str = "http://www.opengis.net/ont/sf#LineString";

    /// `sf:Polygon`.
    pub const POLYGON: &str = "http://www.opengis.net/ont/sf#Polygon";

    /// `sf:MultiPoint`.
    pub const MULTI_POINT: &str = "http://www.opengis.net/ont/sf#MultiPoint";

    /// `sf:MultiLineString`.
    pub const MULTI_LINE_STRING: &str = "http://www.opengis.net/ont/sf#MultiLineString";

    /// `sf:MultiPolygon`.
    pub const MULTI_POLYGON: &str = "http://www.opengis.net/ont/sf#MultiPolygon";

    /// `sf:GeometryCollection`.
    pub const GEOMETRY_COLLECTION: &str = "http://www.opengis.net/ont/sf#GeometryCollection";
}

/// OGC units of measure used by the standard geographic functions.
pub mod uom {
    /// The metre.
    pub const METRE: &str = "http://www.opengis.net/def/uom/OGC/1.0/metre";
    /// The decimal degree.
    pub const DEGREE: &str = "http://www.opengis.net/def/uom/OGC/1.0/degree";
    /// The radian.
    pub const RADIAN: &str = "http://www.opengis.net/def/uom/OGC/1.0/radian";
}
