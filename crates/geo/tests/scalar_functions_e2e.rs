// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The GeoSPARQL 1.1 `geof:` scalar-function family (OGC 22-047r1 Clauses 9–12),
//! driven from real SPARQL query text through `NativeSparqlEngine`.
//!
//! # The property this file pins
//!
//! Two claims, and both are about the *seam* rather than about the geometry.
//!
//! **The scalar seam needs no parser configuration.** Every query below is
//! evaluated by an engine holding [`ParserOptions::default()`] — no extension
//! function namespace, no property function namespace, no IRI list — with
//! `QueryOptions::functions` as the single piece of wiring. A call-position IRI
//! under no configured namespace lowers to `Function::Custom` and is resolved at
//! *evaluation* time, so `geof:sfWithin(?a, ?b)` parses before anything has been
//! registered. That is a real and non-obvious asymmetry with the relation seam
//! (see `query_rewrite_e2e.rs`, which is admission-checked at prepare time and
//! must agree with the registry the plan was prepared against), and it is worth a
//! file of its own because a test that configured the parser "just in case" would
//! pass without proving it.
//!
//! **An unimplemented function aborts the query rather than answering.** Twelve
//! `geof:` functions are registered and deliberately unimplemented. Each has to
//! fail the query *by name*: a `false` from an unimplemented topological
//! predicate, or a plausible-looking polygon from an unimplemented constructor,
//! is indistinguishable from an honest answer and there is nothing downstream
//! that can catch it. The refusal is checked for four of the twelve, each
//! immediately followed by the neighbouring **valid** call of the same query
//! shape — because a seam that refused everything would pass the first half of
//! that claim and be useless.
//!
//! # Why it is a separate file
//!
//! Keeping the two seams apart keeps each file's statement about what a host must
//! configure honest. This one would still pass if the property-function
//! configuration were deleted entirely; that is the point.
//!
//! # Every IRI here is under `example.org`
//!
//! PurRDF mints no vocabulary IRIs. The `geo:`/`geof:` namespaces, the
//! coordinate reference system, the unit IRIs and the Simple Features
//! geometry-type namespace are all the caller's, so the fixtures name
//! `http://example.org/...` throughout and never an OGC IRI.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_geo::geom::Crs;
use purrdf_geo::vocab::{GeoVocab, GeoVocabBuilder};
use purrdf_geo::{GeoTerm, functions};
use purrdf_sparql_algebra::ParserOptions;
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions, UserFunctionRegistry};

// ---------------------------------------------------------------------------
// The caller's vocabulary — a fixture, never a default
// ---------------------------------------------------------------------------

/// The host's `geo:` namespace.
const GEO: &str = "http://example.org/geo#";
/// The host's `geof:` namespace.
const GEOF: &str = "http://example.org/geof/";
/// The one coordinate reference system in play.
///
/// Deliberately used for BOTH the default WKT system and the GeoJSON system: the
/// last test compares a WKT literal against a GeoJSON literal denoting the same
/// geometry, and `purrdf-geo` reprojects nothing, so the comparison is only
/// meaningful if the caller has declared the two serializations to share a
/// system.
const CRS: &str = "http://example.org/crs/planar";
/// A second system, named only as the argument of the unimplemented
/// `geof:transform`.
const CRS_OTHER: &str = "http://example.org/crs/other";
/// The unit IRI the caller declares [`CRS`] to be measured in.
const METRE: &str = "http://example.org/unit/metre";
/// The caller's OGC Simple Features geometry-type namespace.
const SF: &str = "http://example.org/sf#";
/// The fixture data namespace.
const EX: &str = "http://example.org/";

/// `xsd:double`, the datatype every `geof:` measure answers with.
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
/// `xsd:integer`, the datatype `geof:dimension` answers with.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// `xsd:anyURI`, the datatype `geof:geometryType` answers with.
const XSD_ANY_URI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";

/// A four-by-four square at the origin.
const SQUARE: &str = "POLYGON((0 0,4 0,4 4,0 4,0 0))";
/// A two-by-two square at the origin — the constructors' input.
const SMALL_SQUARE: &str = "POLYGON((0 0,2 0,2 2,0 2,0 0))";
/// A point strictly inside [`SQUARE`].
const POINT_INSIDE: &str = "POINT(1 1)";
/// A point well outside [`SQUARE`].
const POINT_OUTSIDE: &str = "POINT(9 9)";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The host's coordinate reference system.
fn crs() -> Crs {
    Crs::new(CRS).expect("a non-empty IRI")
}

/// A fully declared vocabulary: one system, its unit, the metre, and the Simple
/// Features namespace `geof:geometryType` answers from.
fn vocab() -> GeoVocab {
    GeoVocabBuilder::new(GEO, GEOF, crs(), crs())
        .expect("both namespaces are non-empty")
        .declare_crs_unit(&crs(), METRE)
        .expect("a fresh declaration")
        .declare_metre(METRE)
        .expect("a non-empty metre IRI")
        .declare_simple_features_namespace(SF)
        .expect("a non-empty namespace")
        .build()
}

/// The `geof:` family, registered against the host's vocabulary.
fn registry() -> UserFunctionRegistry {
    let mut registry = UserFunctionRegistry::new();
    functions::register(&mut registry, &vocab());
    registry
}

/// The full IRI of a fixture local name.
fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

/// The full IRI of a `geof:` function.
fn geof(local_name: &str) -> String {
    format!("{GEOF}{local_name}")
}

/// The `geo:wktLiteral` datatype IRI, in the host's namespace.
fn wkt_datatype() -> String {
    format!("{GEO}{}", GeoTerm::WktLiteral.local_name())
}

/// The `geo:geoJSONLiteral` datatype IRI, in the host's namespace.
fn geojson_datatype() -> String {
    format!("{GEO}{}", GeoTerm::GeoJsonLiteral.local_name())
}

/// Three spatial objects carrying WKT literals under `ex:geom`: a point inside
/// the square, a point outside it, and the square itself.
///
/// Ordinary data with ordinary predicates — the `geof:` family reads geometry
/// out of *literals* it is handed, so nothing here is `geo:` vocabulary and
/// nothing needs an index.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(&ex("geom"));
    for (subject, lexical) in [("a", POINT_INSIDE), ("b", POINT_OUTSIDE), ("c", SQUARE)] {
        let s = builder.intern_iri(&ex(subject));
        let o = builder.intern_literal(RdfLiteral::typed(lexical, wkt_datatype()));
        builder.push_quad(s, predicate, o, None);
    }
    builder
        .freeze()
        .expect("the fixture is a well-formed dataset")
}

// ---------------------------------------------------------------------------
// Driving the engine
// ---------------------------------------------------------------------------

/// Render a typed literal as SPARQL source text.
fn sparql_literal(lexical: &str, datatype: &str) -> String {
    let escaped = lexical.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"^^<{datatype}>")
}

/// A `geo:wktLiteral` as SPARQL source text.
fn wkt(lexical: &str) -> String {
    sparql_literal(lexical, &wkt_datatype())
}

/// A `geo:geoJSONLiteral` as SPARQL source text.
fn geojson(lexical: &str) -> String {
    sparql_literal(lexical, &geojson_datatype())
}

/// Evaluate `query` with the `geof:` registry as the ONLY configuration.
///
/// The engine is handed [`ParserOptions::default()`] explicitly rather than
/// implicitly: the whole claim of this file is that the scalar seam needs no
/// parse-time declaration, and a default that silently changed would otherwise
/// go unnoticed.
fn run(query: &str) -> Result<Vec<Vec<Option<TermValue>>>, String> {
    let registry = registry();
    let result = NativeSparqlEngine::new()
        .with_parser_options(ParserOptions::default())
        .query_with_options_view(
            &*dataset(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &registry,
                ..QueryOptions::EMPTY
            },
        )
        .map_err(|error| error.to_string())?;
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT returns solutions, got {result:?}");
    };
    Ok(rows)
}

/// [`run`], asserting the query evaluated, and returning its single-column
/// answer as `(lexical form, datatype)` pairs.
fn scalars(query: &str) -> Vec<(String, String)> {
    run(query)
        .unwrap_or_else(|error| {
            panic!("the query must evaluate, but was refused: {error}\n{query}")
        })
        .into_iter()
        .map(|row| match row.into_iter().next().flatten() {
            Some(TermValue::Literal {
                lexical_form,
                datatype,
                ..
            }) => (lexical_form, datatype),
            other => panic!("the projection must be a bound literal, got {other:?}"),
        })
        .collect()
}

/// The single `(lexical form, datatype)` a one-row query answers with.
fn scalar(query: &str) -> (String, String) {
    let rows = scalars(query);
    assert_eq!(
        rows.len(),
        1,
        "the query must answer with exactly one row: {query}"
    );
    rows.into_iter().next().expect("one row")
}

/// Assert a `SELECT (geof:<name>(args…) AS ?v)` answers byte-exactly.
fn assert_projection(local_name: &str, args: &[String], lexical: &str, datatype: &str) {
    let query = format!(
        "SELECT ((<{}>({})) AS ?v) WHERE {{}}",
        geof(local_name),
        args.join(", ")
    );
    let (got_lexical, got_datatype) = scalar(&query);
    assert_eq!(
        got_lexical, lexical,
        "geof:{local_name} answered the wrong lexical form"
    );
    assert_eq!(
        got_datatype, datatype,
        "geof:{local_name} answered the wrong datatype"
    );
}

// ---------------------------------------------------------------------------
// 1 — FILTER
// ---------------------------------------------------------------------------

/// `FILTER(geof:sfWithin(?wkt, ?other))` over a dataset of WKT literals keeps
/// exactly the rows the relation holds for.
///
/// The exact set matters in both directions: `ex:b` must be dropped (its point is
/// outside the square) and `ex:c` must be **kept** (`sfWithin` is reflexive, so
/// the square is within itself). A filter that answered `false` for everything
/// would drop `ex:b` too, and a lower-bound assertion would not notice.
#[test]
fn a_filter_over_a_geof_relation_keeps_exactly_the_rows_the_relation_holds_for() {
    let query = format!(
        "SELECT ?s WHERE {{ \
           ?s <{geom}> ?wkt . \
           <{c}> <{geom}> ?other . \
           FILTER(<{within}>(?wkt, ?other)) \
         }}",
        geom = ex("geom"),
        c = ex("c"),
        within = geof("sfWithin"),
    );
    let mut subjects: Vec<String> = run(&query)
        .expect("the filter query must evaluate")
        .into_iter()
        .map(|row| match row.into_iter().next().flatten() {
            Some(TermValue::Iri(iri)) => iri.strip_prefix(EX).unwrap_or(&iri).to_owned(),
            other => panic!("?s must be a bound IRI, got {other:?}"),
        })
        .collect();
    subjects.sort();
    assert_eq!(
        subjects,
        vec!["a".to_owned(), "c".to_owned()],
        "the point inside the square and the square itself are within it; the point outside is \
         not, and a filter that answered false for everything would also have dropped it"
    );
}

// ---------------------------------------------------------------------------
// 2 — BIND
// ---------------------------------------------------------------------------

/// `BIND(geof:distance(…) AS ?d)` produces the `xsd:double` LEXICAL FORM the
/// canonical mapping specifies, byte-exact.
///
/// The lexical form rather than a numeric comparison is the claim: `geof:`
/// measures are computed as exact rationals and cross a single float boundary on
/// the way out, and the bytes are what a consumer stores and diffs. `5` and
/// `5.0E0` are the same number and different answers.
#[test]
fn a_bind_of_geof_distance_produces_the_exact_xsd_double_lexical_form() {
    let query = format!(
        "SELECT ?d WHERE {{ BIND(<{distance}>({a}, {b}, <{METRE}>) AS ?d) }}",
        distance = geof("distance"),
        a = wkt("POINT(0 0)"),
        b = wkt("POINT(3 4)"),
    );
    assert_eq!(
        scalar(&query),
        ("5.0E0".to_owned(), XSD_DOUBLE.to_owned()),
        "the 3-4-5 triangle's hypotenuse, in the declared unit, as the canonical xsd:double"
    );

    // The neighbouring case that makes the answer about the geometry rather than
    // about the function returning a constant.
    let other = format!(
        "SELECT ?d WHERE {{ BIND(<{distance}>({a}, {b}, <{METRE}>) AS ?d) }}",
        distance = geof("distance"),
        a = wkt("POINT(0 0)"),
        b = wkt("POINT(0 0)"),
    );
    assert_eq!(
        scalar(&other),
        ("0.0E0".to_owned(), XSD_DOUBLE.to_owned()),
        "the distance from a point to itself is zero, and is still rendered canonically"
    );
}

// ---------------------------------------------------------------------------
// 3 — SELECT (expr AS ?v), across the accessor/measure/constructor families
// ---------------------------------------------------------------------------

/// Each of `geof:area`, `geof:envelope`, `geof:boundary`, `geof:convexHull`,
/// `geof:centroid`, `geof:geometryType` and `geof:dimension` answers through real
/// query text with an exact expected literal.
///
/// Every constructor's answer carries the input's own coordinate reference system
/// as an explicit `<IRI>` prefix — `purrdf-geo` reprojects nothing, so the system
/// survives the round trip rather than being re-derived from a default, and the
/// expected strings say so.
#[test]
fn the_geof_accessors_measures_and_constructors_answer_exactly_through_query_text() {
    let wkt_type = wkt_datatype();

    assert_projection(
        "area",
        &[wkt("POLYGON((0 0,1 0,1 1,0 1,0 0))"), format!("<{METRE}>")],
        "1.0E0",
        XSD_DOUBLE,
    );
    assert_projection(
        "envelope",
        &[wkt("LINESTRING(0 0,2 1,1 2)")],
        &format!("<{CRS}> {SMALL_SQUARE}"),
        &wkt_type,
    );
    assert_projection(
        "boundary",
        &[wkt(SMALL_SQUARE)],
        &format!("<{CRS}> MULTILINESTRING((0 0,2 0,2 2,0 2,0 0))"),
        &wkt_type,
    );
    assert_projection(
        "convexHull",
        &[wkt("MULTIPOINT((0 0),(2 0),(2 2),(0 2),(1 1))")],
        &format!("<{CRS}> {SMALL_SQUARE}"),
        &wkt_type,
    );
    assert_projection(
        "centroid",
        &[wkt(SMALL_SQUARE)],
        &format!("<{CRS}> POINT(1 1)"),
        &wkt_type,
    );
    assert_projection(
        "geometryType",
        &[wkt(SMALL_SQUARE)],
        &format!("{SF}Polygon"),
        XSD_ANY_URI,
    );
    assert_projection("dimension", &[wkt(SMALL_SQUARE)], "2", XSD_INTEGER);

    // The neighbouring cases for the two that would look right for a constant
    // implementation: a line is one-dimensional, and its type is not Polygon.
    assert_projection("dimension", &[wkt("LINESTRING(0 0,1 1)")], "1", XSD_INTEGER);
    assert_projection(
        "geometryType",
        &[wkt("LINESTRING(0 0,1 1)")],
        &format!("{SF}LineString"),
        XSD_ANY_URI,
    );
}

// ---------------------------------------------------------------------------
// 4 — the unimplemented twelve abort the query by name
// ---------------------------------------------------------------------------

/// Four of the twelve deliberately-unimplemented `geof:` functions abort a real
/// query, each naming itself — and the identical query shape with
/// `geof:envelope` in the same position succeeds.
///
/// Every one of these has a plausible wrong answer available to it: a
/// `geof:transform` that returned its input's numbers under another system's
/// name, a `geof:buffer` that invented a segment count, a `geof:union` that
/// returned one of its arguments. All of them would be silent. The refusal is
/// the only honest answer, and the valid neighbour is what proves the refusal is
/// about *these four* rather than about the seam being broken.
#[test]
fn the_unimplemented_geof_functions_abort_the_query_by_name_and_the_implemented_one_does_not() {
    let geom = ex("geom");
    let subject = ex("c");
    let bind =
        |call: &str| format!("SELECT ?x WHERE {{ <{subject}> <{geom}> ?g . BIND({call} AS ?x) }}");

    let unimplemented = [
        (
            "transform",
            format!("<{}>(?g, <{CRS_OTHER}>)", geof("transform")),
        ),
        (
            "buffer",
            format!(
                "<{}>(?g, {}, <{METRE}>)",
                geof("buffer"),
                sparql_literal("1", XSD_DOUBLE)
            ),
        ),
        ("union", format!("<{}>(?g, ?g)", geof("union"))),
        (
            "intersection",
            format!("<{}>(?g, ?g)", geof("intersection")),
        ),
    ];

    for (local_name, call) in &unimplemented {
        let rows = run(&bind(call));
        let error = match rows {
            Err(error) => error,
            Ok(answered) => panic!(
                "geof:{local_name} is registered and deliberately unimplemented, so it must \
                 abort the query rather than binding a value — but the query answered {answered:?}"
            ),
        };
        assert!(
            error.contains(local_name),
            "the refusal must name geof:{local_name} so the caller learns which facility is \
             missing, got {error}"
        );
        assert!(
            !error.contains("panicked"),
            "the refusal must be a returned error, never a caught panic: {error}"
        );
    }

    // The neighbouring VALID case: the identical BIND shape, with an implemented
    // constructor in the same position, answers.
    let rows = scalars(&bind(&format!("<{}>(?g)", geof("envelope"))));
    assert_eq!(
        rows,
        vec![(format!("<{CRS}> {SQUARE}"), wkt_datatype())],
        "geof:envelope in the very same BIND position must answer the square's own envelope"
    );
}

// ---------------------------------------------------------------------------
// 5 — one geometry, two serializations, one answer
// ---------------------------------------------------------------------------

/// A GeoJSON literal and a WKT literal denoting the same geometry give the same
/// answer from the same function.
///
/// The datatype decides the codec, and both codecs have to land on the identical
/// exact model or the two serializations are two different geometries wearing one
/// name. Both `geof:asWKT` (which renders the model back out, so a coordinate
/// that decoded differently would show) and `geof:area` (which reads it) are
/// checked, and the negative control is a *different* point, so the equality is
/// not a statement that the function ignores its argument.
#[test]
fn a_geojson_literal_and_a_wkt_literal_of_one_geometry_answer_identically() {
    let point_wkt = wkt("POINT(1 2)");
    let point_geojson = geojson(r#"{"type":"Point","coordinates":[1,2]}"#);
    let square_wkt = wkt(SMALL_SQUARE);
    let square_geojson =
        geojson(r#"{"type":"Polygon","coordinates":[[[0,0],[2,0],[2,2],[0,2],[0,0]]]}"#);

    let as_wkt = |argument: &str| {
        scalar(&format!(
            "SELECT ((<{}>({argument})) AS ?v) WHERE {{}}",
            geof("asWKT")
        ))
    };
    assert_eq!(
        as_wkt(&point_geojson),
        as_wkt(&point_wkt),
        "the two serializations of POINT(1 2) must decode to the identical geometry"
    );
    assert_eq!(
        as_wkt(&point_wkt),
        (format!("<{CRS}> POINT(1 2)"), wkt_datatype()),
        "and that geometry renders with its own coordinate reference system"
    );

    let area = |argument: &str| {
        scalar(&format!(
            "SELECT ((<{}>({argument}, <{METRE}>)) AS ?v) WHERE {{}}",
            geof("area")
        ))
    };
    assert_eq!(
        area(&square_geojson),
        area(&square_wkt),
        "and a measure over the two serializations must agree byte for byte"
    );
    assert_eq!(
        area(&square_wkt),
        ("4.0E0".to_owned(), XSD_DOUBLE.to_owned()),
        "the two-by-two square has area four in the declared unit"
    );

    // The negative control: a different point is a different answer, so the
    // agreements above are about the two codecs rather than about the function
    // ignoring its argument.
    assert_ne!(
        as_wkt(&wkt("POINT(2 1)")),
        as_wkt(&point_wkt),
        "POINT(2 1) and POINT(1 2) are different geometries and must answer differently"
    );
}
