// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! GeoSPARQL's own SHACL shapes validate a geo dataset through `purrdf-shapes`,
//! with no changes required to `purrdf-shapes` itself.
//!
//! # The property this file pins
//!
//! GeoSPARQL 1.1 ships a SHACL validator alongside the ontology. Its shapes are
//! **SHACL Core only** — `sh:datatype`, `sh:minCount`, `sh:maxCount`,
//! `sh:pattern`, `sh:class`, `sh:nodeKind`, `sh:not`, `sh:or`,
//! `sh:lessThanOrEquals`, `sh:targetObjectsOf`, `sh:targetSubjectsOf` and
//! `sh:targetClass` — which is exactly the surface `purrdf-shapes` already
//! implements. This crate's second acceptance criterion is therefore an
//! *integration* claim rather than a feature claim: the two crates meet at
//! `purrdf-shapes`' existing public entry points and nothing has to be added to
//! either of them.
//!
//! # Why the shapes here are first-party rather than the shipped OGC file
//!
//! PurRDF mints no vocabulary IRIs and its fixtures live under `example.org`, so
//! embedding the OGC validator verbatim would smuggle a real ontology's IRIs into
//! a test fixture and quietly assert a default namespace this crate does not
//! have. The shapes graph below is written in `example.org` space and is
//! **structurally the same shapes**, mirroring the shipped validator's:
//!
//! | mirrored | claim |
//! |---|---|
//! | S1 | an object of `geo:hasGeometry` carries at least one serialization, and at most one `geo:asWKT` |
//! | S2 | an object of `geo:hasGeometry` is not itself the subject of one — `geo:Feature` and `geo:Geometry` are disjoint |
//! | S3 | the object of `geo:hasSerialization` is a literal |
//! | S4 | the object of `geo:asWKT` carries the `geo:wktLiteral` datatype |
//! | S09–S15 | at most one `geo:dimension` / `geo:coordinateDimension` / `geo:isEmpty` / `geo:isSimple` per geometry, each of its declared datatype |
//! | S16 | a `geo:wktLiteral`'s lexical form matches the shipped validator's regex `^\s*$\|^\s*(M\|P\|C\|S\|L\|T\|<\|m\|p\|c\|s\|l\|t)` |
//! | S21 | `geo:dimension` is at most `geo:coordinateDimension` |
//!
//! The two typing shapes (`sh:targetSubjectsOf` + `sh:class geo:Feature`,
//! `sh:class geo:Geometry`) mirror the validator's class-membership shapes and
//! are what put `sh:targetSubjectsOf` and `sh:class` in play here.
//!
//! # What makes this a test about GeoSPARQL rather than about SHACL
//!
//! A shapes file that passed cleanly over geometry literals `purrdf-geo` cannot
//! read would prove nothing about GeoSPARQL: it would be a SHACL smoke test
//! wearing geospatial vocabulary. So the conforming fixture's geometry literals
//! are held in one Rust table, the N-Triples are rendered from it, and
//! [`the_conforming_fixtures_geometry_literals_are_ones_purrdf_geo_actually_parses`]
//! feeds every one of them back through [`purrdf_geo::wkt::parse`]. The shapes and
//! the parser are cross-checked against the same bytes.
//!
//! # Why it is a separate file
//!
//! It shares no machinery with the two SPARQL end-to-end files: no engine, no
//! registry, no `GeoVocab`, no index. The acceptance criterion it closes is about
//! the `purrdf-shapes` seam, and mixing it into a file that stands up a SPARQL
//! evaluator would make it impossible to see that the seam needs none of that.

use std::collections::BTreeSet;

use purrdf_geo::geom::Crs;
use purrdf_geo::wkt;
use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::text_ingest::parse_ntriples_to_dataset;

// ---------------------------------------------------------------------------
// Namespaces — every one under example.org
// ---------------------------------------------------------------------------

/// The caller's `geo:` namespace. A fixture, never a default.
const GEO: &str = "http://example.org/geo#";
/// The fixture data namespace.
const EX: &str = "http://example.org/";
/// The shapes graph's own namespace.
const SHAPES_NS: &str = "http://example.org/shapes#";
/// The coordinate reference system the fixture geometries are expressed in.
const CRS: &str = "http://example.org/crs/planar";
/// `rdf:type`.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// The `xsd:` namespace.
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
/// The `sh:` namespace, for the constraint-component IRIs the report names.
const SH: &str = "http://www.w3.org/ns/shacl#";

// ---------------------------------------------------------------------------
// The shapes graph
// ---------------------------------------------------------------------------

/// The first-party mirror of the shipped GeoSPARQL SHACL validator.
///
/// SHACL Core throughout: there is no `sh:sparql`, no SHACL-AF node expression
/// and no `sh:js` anywhere in it, which is why the criterion is that it validates
/// through the *existing* `purrdf-shapes` surface.
fn shapes_graph() -> String {
    format!(
        r#"
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix xsd:  <{XSD}> .
@prefix geo:  <{GEO}> .
@prefix exs:  <{SHAPES_NS}> .

# S1, S2, S09-S15, S21 and the geometry typing shape, on every object of
# geo:hasGeometry.
exs:GeometryShape
    a sh:NodeShape ;
    sh:targetObjectsOf geo:hasGeometry ;
    sh:class geo:Geometry ;
    # S1a - at least one serialization, by any of the serialization properties.
    sh:or (
        [ sh:property [ sh:path geo:hasSerialization ; sh:minCount 1 ] ]
        [ sh:property [ sh:path geo:asWKT           ; sh:minCount 1 ] ]
        [ sh:property [ sh:path geo:asGeoJSON       ; sh:minCount 1 ] ]
    ) ;
    # S1b - and at most one geo:asWKT.
    sh:property [ sh:path geo:asWKT ; sh:maxCount 1 ] ;
    # S2 - a Geometry is not a Feature, so it is not the subject of hasGeometry.
    sh:not [ sh:property [ sh:path geo:hasGeometry ; sh:minCount 1 ] ] ;
    # S09-S15 - at most one of each geometry property, each of its datatype.
    sh:property [ sh:path geo:dimension           ; sh:maxCount 1 ; sh:datatype xsd:integer ] ;
    sh:property [ sh:path geo:coordinateDimension ; sh:maxCount 1 ; sh:datatype xsd:integer ] ;
    sh:property [ sh:path geo:isEmpty             ; sh:maxCount 1 ; sh:datatype xsd:boolean ] ;
    sh:property [ sh:path geo:isSimple            ; sh:maxCount 1 ; sh:datatype xsd:boolean ] ;
    # S21 - the topological dimension cannot exceed the coordinate dimension.
    sh:property [ sh:path geo:dimension ; sh:lessThanOrEquals geo:coordinateDimension ] .

# The feature typing shape, on every subject of geo:hasGeometry.
exs:FeatureShape
    a sh:NodeShape ;
    sh:targetSubjectsOf geo:hasGeometry ;
    sh:class geo:Feature .

# S3 - the object of geo:hasSerialization is a literal.
exs:SerializationShape
    a sh:NodeShape ;
    sh:targetObjectsOf geo:hasSerialization ;
    sh:nodeKind sh:Literal .

# S4 and S16 - the object of geo:asWKT carries the geo:wktLiteral datatype, and
# its lexical form matches the shipped validator's own regex.
exs:WktLiteralShape
    a sh:NodeShape ;
    sh:targetObjectsOf geo:asWKT ;
    sh:datatype geo:wktLiteral ;
    sh:pattern "^\\s*$|^\\s*(M|P|C|S|L|T|<|m|p|c|s|l|t)" .
"#
    )
}

// ---------------------------------------------------------------------------
// The conforming fixture
// ---------------------------------------------------------------------------

/// One geometry node of the conforming fixture.
///
/// The geometry literals live here, in one table, rather than inside a
/// hand-written N-Triples blob: the N-Triples are rendered from this table and
/// [`the_conforming_fixtures_geometry_literals_are_ones_purrdf_geo_actually_parses`]
/// reads the same table, so the shapes and `purrdf-geo`'s WKT parser are
/// cross-checked against literally the same bytes.
#[derive(Clone, Copy, Debug)]
struct GeometryNode {
    /// The feature's local name.
    feature: &'static str,
    /// The geometry's local name.
    geometry: &'static str,
    /// The serialization property's `geo:` local name.
    serialization: &'static str,
    /// The `geo:wktLiteral` lexical form.
    lexical: &'static str,
    /// The value of `geo:dimension`.
    dimension: u32,
    /// The value of `geo:coordinateDimension`.
    coordinate_dimension: u32,
}

/// The conforming map: a point, a polygon and a line.
///
/// The line is serialized through `geo:hasSerialization` rather than
/// `geo:asWKT`, so the `sh:or` of S1 is exercised on more than one branch and the
/// S3 literal check has something to target.
const CONFORMING: [GeometryNode; 3] = [
    GeometryNode {
        feature: "f1",
        geometry: "g1",
        serialization: "asWKT",
        lexical: "POINT(1 1)",
        dimension: 0,
        coordinate_dimension: 2,
    },
    GeometryNode {
        feature: "f2",
        geometry: "g2",
        serialization: "asWKT",
        lexical: "POLYGON((0 0,4 0,4 4,0 4,0 0))",
        dimension: 2,
        coordinate_dimension: 2,
    },
    GeometryNode {
        feature: "f3",
        geometry: "g3",
        serialization: "hasSerialization",
        lexical: "LINESTRING(0 0,2 2)",
        dimension: 1,
        coordinate_dimension: 2,
    },
];

/// The full IRI of a fixture local name.
fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

/// The full IRI of a `geo:` local name.
fn geo(local: &str) -> String {
    format!("{GEO}{local}")
}

/// One N-Triples statement with an IRI object.
fn iri_triple(subject: &str, predicate: &str, object: &str) -> String {
    format!("<{subject}> <{predicate}> <{object}> .\n")
}

/// One N-Triples statement with a typed-literal object.
fn literal_triple(subject: &str, predicate: &str, lexical: &str, datatype: &str) -> String {
    let escaped = lexical.replace('\\', "\\\\").replace('"', "\\\"");
    format!("<{subject}> <{predicate}> \"{escaped}\"^^<{datatype}> .\n")
}

/// The conforming dataset, rendered from [`CONFORMING`].
fn conforming_ntriples() -> String {
    let mut out = String::new();
    for node in CONFORMING {
        let feature = ex(node.feature);
        let geometry = ex(node.geometry);
        out.push_str(&iri_triple(&feature, RDF_TYPE, &geo("Feature")));
        out.push_str(&iri_triple(&feature, &geo("hasGeometry"), &geometry));
        out.push_str(&iri_triple(&geometry, RDF_TYPE, &geo("Geometry")));
        out.push_str(&literal_triple(
            &geometry,
            &geo(node.serialization),
            node.lexical,
            &geo("wktLiteral"),
        ));
        out.push_str(&literal_triple(
            &geometry,
            &geo("dimension"),
            &node.dimension.to_string(),
            &format!("{XSD}integer"),
        ));
        out.push_str(&literal_triple(
            &geometry,
            &geo("coordinateDimension"),
            &node.coordinate_dimension.to_string(),
            &format!("{XSD}integer"),
        ));
        out.push_str(&literal_triple(
            &geometry,
            &geo("isEmpty"),
            "false",
            &format!("{XSD}boolean"),
        ));
        out.push_str(&literal_triple(
            &geometry,
            &geo("isSimple"),
            "true",
            &format!("{XSD}boolean"),
        ));
    }
    out
}

/// A minimal conforming geometry node, as the base every violation fixture
/// perturbs in exactly one way.
///
/// Written out rather than reusing [`conforming_ntriples`] because a violation
/// fixture has to be *focused*: if the base carried three geometries, a report
/// with one violation would not prove which shape produced it.
fn base_geometry(feature: &str, geometry: &str) -> String {
    let feature = ex(feature);
    let geometry = ex(geometry);
    let mut out = String::new();
    out.push_str(&iri_triple(&feature, RDF_TYPE, &geo("Feature")));
    out.push_str(&iri_triple(&feature, &geo("hasGeometry"), &geometry));
    out.push_str(&iri_triple(&geometry, RDF_TYPE, &geo("Geometry")));
    out
}

/// `base_geometry` plus a well-formed `geo:asWKT` serialization.
fn base_with_wkt(feature: &str, geometry: &str) -> String {
    let mut out = base_geometry(feature, geometry);
    out.push_str(&literal_triple(
        &ex(geometry),
        &geo("asWKT"),
        "POINT(1 1)",
        &geo("wktLiteral"),
    ));
    out
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// The `(focus node, result path, constraint component)` of one violation, each
/// rendered in N-Triples term syntax.
type Finding = (String, Option<String>, String);

/// Validate `data_ntriples` against [`shapes_graph`] and return `(conforms,
/// findings)`.
///
/// This is the whole integration surface: `parse_shapes` to read the shapes
/// graph, `validate_dataset_with_shapes_graph` to run it. Nothing else from
/// `purrdf-shapes` is needed, and nothing in `purrdf-shapes` had to change.
fn validate(data_ntriples: &str) -> (bool, BTreeSet<Finding>) {
    let shapes = parse_shapes(&shapes_graph(), None).expect("the mirrored shapes graph parses");
    let data = parse_ntriples_to_dataset(data_ntriples)
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")));
    let report = validate_dataset_with_shapes_graph(&data, &shapes, None)
        .expect("validation runs to a verdict");
    let findings = report
        .result_tuples()
        .into_iter()
        .map(|(focus, path, _value, component, _shape, _severity)| (focus, path, component))
        .collect();
    (report.conforms, findings)
}

/// A focus-node/path/component expectation.
fn finding(focus: &str, path: Option<&str>, component: &str) -> Finding {
    (
        format!("<{focus}>"),
        path.map(|iri| format!("<{iri}>")),
        format!("<{SH}{component}>"),
    )
}

/// The same, when the focus node is a literal rather than a node.
///
/// `xsd:string` is written without its datatype suffix, which is the N-Triples
/// simple-literal abbreviation the report's term rendering uses. The suffix is
/// present for every other datatype, so the two spellings are distinguished here
/// rather than papered over — an expectation that dropped the datatype
/// unconditionally would stop noticing a wrong one.
fn literal_finding(lexical: &str, datatype: &str, component: &str) -> Finding {
    let rendered = if datatype == format!("{XSD}string") {
        format!("\"{lexical}\"")
    } else {
        format!("\"{lexical}\"^^<{datatype}>")
    };
    (rendered, None, format!("<{SH}{component}>"))
}

/// Assert `data` is refused, and refused for exactly `expected`.
///
/// The expected findings are asserted as a whole set rather than by membership:
/// a test that only checked `!conforms` would pass for a violation of a
/// completely different shape, which is precisely the failure mode of a
/// per-shape test suite that shares one fixture.
fn assert_violations(what: &str, data: &str, expected: &[Finding]) {
    let (conforms, findings) = validate(data);
    assert!(
        !conforms,
        "{what}: the shapes graph must refuse this dataset"
    );
    let expected: BTreeSet<Finding> = expected.iter().cloned().collect();
    assert_eq!(
        findings, expected,
        "{what}: the report must name exactly the expected focus node, path and constraint \
         component"
    );
}

// ---------------------------------------------------------------------------
// 1 — the conforming dataset
// ---------------------------------------------------------------------------

/// A conforming geo dataset validates clean through `purrdf-shapes`: it conforms
/// **and** the report holds zero results.
///
/// Both halves are asserted because they are separable claims. `conforms` is a
/// boolean an implementation could compute from something other than the result
/// set; a report that carried warnings or infos while still reporting `conforms:
/// true` would satisfy one and not the other, and a caller reading the results
/// would see noise the verdict denied.
#[test]
fn a_conforming_geo_dataset_validates_clean_through_purrdf_shapes() {
    let (conforms, findings) = validate(&conforming_ntriples());
    assert!(
        conforms,
        "the conforming fixture must satisfy the mirrored GeoSPARQL shapes: {findings:?}"
    );
    assert!(
        findings.is_empty(),
        "a conforming verdict must come with an empty result set: {findings:?}"
    );
}

/// The conforming fixture is not vacuous: the shapes really do have focus nodes
/// to run over.
///
/// Without this, a shapes graph whose targets matched nothing would produce the
/// identical clean report as one that validated three geometries, and the test
/// above would pass for the wrong reason. Making one geometry invalid must
/// therefore make the very same fixture fail.
#[test]
fn the_conforming_fixture_is_not_vacuous_because_perturbing_it_makes_it_fail() {
    let perturbed = conforming_ntriples().replace("\"POINT(1 1)\"", "\"not a geometry at all\"");
    assert_ne!(
        perturbed,
        conforming_ntriples(),
        "the perturbation must actually change the fixture"
    );
    let (conforms, findings) = validate(&perturbed);
    assert!(
        !conforms,
        "if the shapes matched no focus nodes, a broken literal would still validate clean"
    );
    assert!(
        !findings.is_empty(),
        "the refusal must come with at least one result: {findings:?}"
    );
}

// ---------------------------------------------------------------------------
// 2 — each shape catches its own violation
// ---------------------------------------------------------------------------

/// **S1a**: a geometry with no serialization at all is refused, at the geometry,
/// by the `sh:or` over the serialization properties.
#[test]
fn s1_a_geometry_with_no_serialization_is_refused_at_the_geometry() {
    assert_violations(
        "S1a (no serialization)",
        &base_geometry("f1", "g1"),
        &[finding(&ex("g1"), None, "OrConstraintComponent")],
    );
}

/// **S1b**: a geometry with two `geo:asWKT` serializations is refused, at the
/// geometry, on the `geo:asWKT` path.
///
/// The neighbouring valid case is the conforming fixture's single-`asWKT`
/// geometry, which the clean-validation test above already exercises.
#[test]
fn s1_a_geometry_with_two_wkt_serializations_is_refused_on_the_as_wkt_path() {
    let mut data = base_with_wkt("f1", "g1");
    data.push_str(&literal_triple(
        &ex("g1"),
        &geo("asWKT"),
        "POINT(2 2)",
        &geo("wktLiteral"),
    ));
    assert_violations(
        "S1b (two geo:asWKT)",
        &data,
        &[finding(
            &ex("g1"),
            Some(&geo("asWKT")),
            "MaxCountConstraintComponent",
        )],
    );
}

/// **S2**: a node that is both the object and the subject of `geo:hasGeometry` is
/// refused at that node — `geo:Feature` and `geo:Geometry` are disjoint.
#[test]
fn s2_a_geometry_that_is_also_a_feature_is_refused_at_that_node() {
    let mut data = base_with_wkt("f1", "g1");
    // `ex:g1` now hangs a geometry of its own, which makes it a Feature as well
    // as a Geometry.
    data.push_str(&iri_triple(&ex("g1"), &geo("hasGeometry"), &ex("g2")));
    data.push_str(&iri_triple(&ex("g2"), RDF_TYPE, &geo("Geometry")));
    data.push_str(&literal_triple(
        &ex("g2"),
        &geo("asWKT"),
        "POINT(3 3)",
        &geo("wktLiteral"),
    ));
    // `ex:g1` is now also a subject of geo:hasGeometry, so the feature typing
    // shape targets it too — and it is not typed geo:Feature.
    assert_violations(
        "S2 (a Geometry that is also a Feature)",
        &data,
        &[
            finding(&ex("g1"), None, "NotConstraintComponent"),
            finding(&ex("g1"), None, "ClassConstraintComponent"),
        ],
    );
}

/// **S3**: an IRI in the object position of `geo:hasSerialization` is refused at
/// that IRI, by `sh:nodeKind sh:Literal`.
///
/// A serialization is a lexical form; an IRI there names something else entirely
/// and would be read by no codec.
#[test]
fn s3_a_non_literal_serialization_is_refused_at_the_offending_object() {
    let mut data = base_geometry("f1", "g1");
    data.push_str(&iri_triple(
        &ex("g1"),
        &geo("hasSerialization"),
        &ex("notALiteral"),
    ));
    assert_violations(
        "S3 (non-literal serialization)",
        &data,
        &[finding(
            &ex("notALiteral"),
            None,
            "NodeKindConstraintComponent",
        )],
    );
}

/// **S4**: a `geo:asWKT` object carrying `xsd:string` rather than
/// `geo:wktLiteral` is refused at that literal.
///
/// The lexical form is fine — it is the same `POINT(1 1)` the conforming fixture
/// uses, so the S16 pattern is satisfied and only the datatype check can fire.
/// A store that lost the datatype has lost the fact that the characters are a
/// geometry.
#[test]
fn s4_a_wkt_serialization_with_the_wrong_datatype_is_refused_at_the_literal() {
    let mut data = base_geometry("f1", "g1");
    data.push_str(&literal_triple(
        &ex("g1"),
        &geo("asWKT"),
        "POINT(1 1)",
        &format!("{XSD}string"),
    ));
    assert_violations(
        "S4 (geo:asWKT with xsd:string)",
        &data,
        &[literal_finding(
            "POINT(1 1)",
            &format!("{XSD}string"),
            "DatatypeConstraintComponent",
        )],
    );
}

/// **S09–S15**: a geometry with two `geo:dimension` values is refused at the
/// geometry, on the `geo:dimension` path.
///
/// Both values are within the coordinate dimension, so S21 cannot fire and the
/// maximum-cardinality check is the only thing that can produce this result.
#[test]
fn s09_to_s15_a_geometry_with_two_dimensions_is_refused_on_the_dimension_path() {
    let mut data = base_with_wkt("f1", "g1");
    for value in ["0", "1"] {
        data.push_str(&literal_triple(
            &ex("g1"),
            &geo("dimension"),
            value,
            &format!("{XSD}integer"),
        ));
    }
    data.push_str(&literal_triple(
        &ex("g1"),
        &geo("coordinateDimension"),
        "2",
        &format!("{XSD}integer"),
    ));
    assert_violations(
        "S09-S15 (two geo:dimension values)",
        &data,
        &[finding(
            &ex("g1"),
            Some(&geo("dimension")),
            "MaxCountConstraintComponent",
        )],
    );
}

/// **S16**: a `geo:wktLiteral` whose lexical form does not match the shipped
/// validator's regex is refused at that literal.
///
/// The regex admits only whitespace, or whitespace followed by a WKT keyword's
/// initial or a `<` coordinate-reference-system prefix. A literal starting with a
/// digit is outside it.
#[test]
fn s16_a_wkt_literal_with_a_malformed_lexical_form_is_refused_at_the_literal() {
    let mut data = base_geometry("f1", "g1");
    data.push_str(&literal_triple(
        &ex("g1"),
        &geo("asWKT"),
        "999 not a geometry",
        &geo("wktLiteral"),
    ));
    assert_violations(
        "S16 (malformed wktLiteral lexical form)",
        &data,
        &[literal_finding(
            "999 not a geometry",
            &geo("wktLiteral"),
            "PatternConstraintComponent",
        )],
    );

    // The neighbouring VALID cases, so the pattern is not simply refusing
    // everything: an explicit CRS prefix and a lower-case keyword both match.
    for lexical in [
        &format!("<{CRS}> POINT(1 1)"),
        "linestring(0 0,1 1)",
        "MULTIPOINT((0 0))",
    ] {
        let mut valid = base_geometry("f1", "g1");
        valid.push_str(&literal_triple(
            &ex("g1"),
            &geo("asWKT"),
            lexical,
            &geo("wktLiteral"),
        ));
        let (conforms, findings) = validate(&valid);
        assert!(
            conforms,
            "the S16 pattern must admit the well-formed literal {lexical:?}: {findings:?}"
        );
    }
}

/// **S21**: a geometry whose `geo:dimension` exceeds its
/// `geo:coordinateDimension` is refused at the geometry, on the `geo:dimension`
/// path.
#[test]
fn s21_a_dimension_greater_than_the_coordinate_dimension_is_refused() {
    let mut data = base_with_wkt("f1", "g1");
    data.push_str(&literal_triple(
        &ex("g1"),
        &geo("dimension"),
        "3",
        &format!("{XSD}integer"),
    ));
    data.push_str(&literal_triple(
        &ex("g1"),
        &geo("coordinateDimension"),
        "2",
        &format!("{XSD}integer"),
    ));
    assert_violations(
        "S21 (dimension > coordinateDimension)",
        &data,
        &[finding(
            &ex("g1"),
            Some(&geo("dimension")),
            "LessThanOrEqualsConstraintComponent",
        )],
    );

    // The neighbouring VALID case: equality satisfies `<=`, and a two-dimensional
    // geometry with two ordinates is the commonest real datum there is.
    let mut valid = base_with_wkt("f1", "g1");
    for (property, value) in [("dimension", "2"), ("coordinateDimension", "2")] {
        valid.push_str(&literal_triple(
            &ex("g1"),
            &geo(property),
            value,
            &format!("{XSD}integer"),
        ));
    }
    let (conforms, findings) = validate(&valid);
    assert!(
        conforms,
        "dimension == coordinateDimension satisfies sh:lessThanOrEquals: {findings:?}"
    );
}

// ---------------------------------------------------------------------------
// 3 — the literals are ones purrdf-geo actually reads
// ---------------------------------------------------------------------------

/// Every geometry literal in the conforming fixture parses through
/// [`purrdf_geo::wkt::parse`].
///
/// This is what makes the criterion above a GeoSPARQL claim rather than a SHACL
/// one. A shapes graph is free to be satisfied by literals no geometry engine
/// can read — `sh:pattern` only inspects the first non-space character — so a
/// clean validation over unparseable geometries would prove nothing about
/// `purrdf-geo`. The literals are drawn from the same [`CONFORMING`] table the
/// N-Triples are rendered from, so there is no second copy to drift.
#[test]
fn the_conforming_fixtures_geometry_literals_are_ones_purrdf_geo_actually_parses() {
    let crs = Crs::new(CRS).expect("a non-empty IRI");
    let rendered = conforming_ntriples();
    for node in CONFORMING {
        let parsed = wkt::parse(node.lexical, &crs).unwrap_or_else(|error| {
            panic!(
                "the conforming fixture's {} literal {:?} must be one purrdf-geo reads, but it \
                 was refused: {error}",
                node.geometry, node.lexical
            )
        });
        assert_eq!(
            parsed.crs().as_str(),
            CRS,
            "a literal with no explicit prefix takes the caller's default system"
        );
        // And the very bytes that were parsed are the bytes the validated
        // dataset carried, so the two checks are about one fixture.
        assert!(
            rendered.contains(&format!("\"{}\"^^<{}>", node.lexical, geo("wktLiteral"))),
            "the parsed literal must be the one the shapes validated: {:?}",
            node.lexical
        );
    }

    // The negative control: the lexical form the S16 test refuses is one
    // purrdf-geo also refuses, so the pattern and the parser agree about which
    // bytes are a geometry.
    assert!(
        wkt::parse("999 not a geometry", &crs).is_err(),
        "the literal the S16 shape refuses must be one purrdf-geo cannot read either"
    );
}

// ---------------------------------------------------------------------------
// 4 — the integration point is the existing public surface
// ---------------------------------------------------------------------------

/// The whole integration needs exactly two `purrdf-shapes` entry points, and no
/// change to `purrdf-shapes` itself.
///
/// The body below is the complete wiring, written out inline rather than routed
/// through this file's helpers so that it can be read as the list of APIs it
/// touches:
///
/// * [`purrdf_shapes::engine::parse_shapes`] — read the shapes graph.
/// * [`purrdf_shapes::engine::validate_dataset_with_shapes_graph`] — run it.
///
/// `purrdf_shapes::text_ingest::parse_ntriples_to_dataset` appears too, but only
/// to build the *fixture*: a host arrives with an `RdfDataset` it already has, so
/// that call is a property of this test rather than of the integration. Nothing
/// else is used: no SHACL-AF surface, no governed entry, no shapes-graph
/// rewriting, no custom constraint component, and — the substance of the
/// criterion — no new API. This test cannot mechanically prove the absence of a
/// change to another crate; what it pins is that the shapes above are satisfied
/// by the surface that already exists, which is the observable half of that
/// claim.
#[test]
fn the_integration_needs_only_parse_shapes_and_validate_dataset_with_shapes_graph() {
    let shapes =
        parse_shapes(&shapes_graph(), None).expect("SHACL Core parses on the existing surface");
    let data = parse_ntriples_to_dataset(&conforming_ntriples())
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")));
    let report = validate_dataset_with_shapes_graph(&data, &shapes, None)
        .expect("the existing ungoverned validation entry runs GeoSPARQL's shapes");

    assert!(
        report.conforms,
        "the geo dataset conforms: {:?}",
        report.results
    );
    assert!(
        report.results.is_empty(),
        "and the report is empty: {:?}",
        report.results
    );
}
