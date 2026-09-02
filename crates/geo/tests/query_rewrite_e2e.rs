// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The GeoSPARQL 1.1 **Query Rewrite** extension (OGC 22-047r1 Clause 13),
//! driven from real SPARQL query text through `NativeSparqlEngine`.
//!
//! # The property this file pins
//!
//! This crate's headline acceptance criterion is that *at least one `geo:`
//! predicate is rewritten via query rewrite through the property-function seam,
//! covered by an end-to-end test*. "End to end" is load-bearing: `purrdf-geo`'s
//! own in-crate tests build a [`GeoRelation`](purrdf_geo::relation::GeoRelation)
//! and drive `PropertyFunction::open` directly, which proves the relation
//! computes the right rows but proves nothing about whether a host can reach it
//! from a query. Between the relation and a host sit the parser (which must
//! claim `geo:sfWithin` in predicate position rather than lowering it to an
//! ordinary triple pattern), the planner (which admits the call against the
//! registry the plan was prepared under) and the evaluator (which joins the
//! relation's rows back into the surrounding basic graph pattern). Every one of
//! those stages fails *silently* when it is wrong — a predicate that stayed an
//! ordinary triple pattern matches nothing and answers the empty bag with no
//! diagnostic — so the only way to know the seam is wired is to write the query
//! text and check the rows.
//!
//! # Why it is a separate file from the scalar-function end-to-end tests
//!
//! The two GeoSPARQL seams have opposite wiring requirements, and conflating
//! them would let one file's configuration mask the other's gap. The relation
//! seam is **parse-time and admission-time** configuration: an IRI has to be
//! recognized in predicate position ([`ParserOptions::property_fn_iris`]) and the
//! prepared plan carries the identity of the registry it was admitted against.
//! The `geof:` scalar seam needs no parser configuration at all. Keeping them in
//! separate files keeps each file's claim about *what a host must configure*
//! honest; `scalar_functions_e2e.rs` is the sibling.
//!
//! # Every IRI here is under `example.org`
//!
//! PurRDF mints no vocabulary IRIs, and `GeoVocab` exists precisely so the
//! `geo:`/`geof:` namespaces and the coordinate reference system are the
//! caller's. Embedding the real OGC IRIs in a fixture would quietly assert a
//! default that this crate does not have, so the fixtures use
//! `http://example.org/geo#`, `http://example.org/geof/` and
//! `http://example.org/crs/planar`.

use std::sync::Arc;

use purrdf_core::{
    DatasetView, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_geo::geom::Crs;
use purrdf_geo::relation::{GeoIndex, GeoIndexConfig, GraphSelector, register};
use purrdf_geo::vocab::{GeoVocab, GeoVocabBuilder};
use purrdf_geo::{GeoTerm, RelationFamily};
use purrdf_sparql_algebra::ParserOptions;
use purrdf_sparql_eval::{NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions};

// ---------------------------------------------------------------------------
// The caller's vocabulary — a fixture, never a default
// ---------------------------------------------------------------------------

/// The host's `geo:` namespace.
const GEO: &str = "http://example.org/geo#";
/// The host's `geof:` namespace. Unused by the relation seam, but a `GeoVocab`
/// carries both, and naming it here keeps the fixture a complete configuration.
const GEOF: &str = "http://example.org/geof/";
/// The coordinate reference system every fixture geometry is expressed in.
const CRS: &str = "http://example.org/crs/planar";
/// The fixture data namespace.
const EX: &str = "http://example.org/";
/// `xsd:string`, for the `ex:name` labels the join test reads.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// A four-by-four square at the origin.
const SQUARE: &str = "POLYGON((0 0,4 0,4 4,0 4,0 0))";
/// A point strictly inside [`SQUARE`].
const POINT_INSIDE: &str = "POINT(1 1)";
/// A point well outside [`SQUARE`].
const POINT_OUTSIDE: &str = "POINT(9 9)";
/// A second point strictly inside [`SQUARE`], carried by a **bare** geometry —
/// a `geo:Geometry` with `geo:asWKT` directly on it and no feature above it.
const POINT_BARE: &str = "POINT(2 2)";

// ---------------------------------------------------------------------------
// Fixture construction
// ---------------------------------------------------------------------------

/// The object of a fixture triple.
#[derive(Clone, Debug)]
enum Object {
    /// An IRI object.
    Iri(String),
    /// A literal object: lexical form and datatype IRI.
    Literal {
        /// The lexical form.
        lexical: String,
        /// The datatype IRI.
        datatype: String,
    },
}

/// One fixture triple, in dataset-independent value space.
#[derive(Clone, Debug)]
struct Triple {
    /// The subject IRI.
    subject: String,
    /// The predicate IRI.
    predicate: String,
    /// The object.
    object: Object,
}

/// The full IRI of a fixture local name.
fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

/// The full IRI of a `geo:` local name, in the host's namespace.
fn geo(term: GeoTerm) -> String {
    format!("{GEO}{}", term.local_name())
}

/// The full IRI of a `geo:` spatial relation, in the host's namespace.
fn geo_relation(local: &str) -> String {
    format!("{GEO}{local}")
}

/// A triple, with the subject spelled as a fixture local name.
fn triple(subject: &str, predicate: String, object: Object) -> Triple {
    Triple {
        subject: ex(subject),
        predicate,
        object,
    }
}

/// A `geo:wktLiteral` object.
fn wkt(lexical: &str) -> Object {
    Object::Literal {
        lexical: lexical.to_owned(),
        datatype: geo(GeoTerm::WktLiteral),
    }
}

/// An `xsd:string` object.
fn label(lexical: &str) -> Object {
    Object::Literal {
        lexical: lexical.to_owned(),
        datatype: XSD_STRING.to_owned(),
    }
}

/// An IRI object, spelled as a fixture local name.
fn node(local: &str) -> Object {
    Object::Iri(ex(local))
}

/// The small map every test below reads.
///
/// Three features, each with a default geometry — a point inside the square, a
/// point outside it, and the square itself — plus one **bare** `geo:Geometry`
/// (`ex:gBare`) carrying `geo:asWKT` directly with no feature above it. The bare
/// geometry is what makes the geometry/geometry and geometry/feature branches of
/// the Clause 13 rule observable from query text rather than merely inferable.
fn map_triples() -> Vec<Triple> {
    vec![
        triple("fInside", geo(GeoTerm::HasDefaultGeometry), node("gInside")),
        triple("fInside", ex("name"), label("inside")),
        triple("gInside", geo(GeoTerm::AsWkt), wkt(POINT_INSIDE)),
        triple(
            "fOutside",
            geo(GeoTerm::HasDefaultGeometry),
            node("gOutside"),
        ),
        triple("fOutside", ex("name"), label("outside")),
        triple("gOutside", geo(GeoTerm::AsWkt), wkt(POINT_OUTSIDE)),
        triple("fSquare", geo(GeoTerm::HasDefaultGeometry), node("gSquare")),
        triple("fSquare", ex("name"), label("square")),
        triple("gSquare", geo(GeoTerm::AsWkt), wkt(SQUARE)),
        triple("gBare", geo(GeoTerm::AsWkt), wkt(POINT_BARE)),
    ]
}

/// [`map_triples`] plus three **asserted** statements.
///
/// `:-` in the Clause 13 rules is an entailment, not a definition, so an
/// asserted `geo:sfWithin` matches whether or not the geometries satisfy it.
/// `ex:fOutside geo:sfWithin ex:fSquare` is refuted by the geometry and must
/// still appear; `ex:fInside geo:sfWithin ex:fSquare` is *also* computed and
/// must appear exactly once. The `geo:rcc8ec` statement is the ordinary-data
/// control: RCC8 is not a registered family here, so that predicate stays an
/// ordinary triple pattern.
fn asserted_triples() -> Vec<Triple> {
    let mut triples = map_triples();
    triples.push(triple(
        "fOutside",
        geo_relation("sfWithin"),
        node("fSquare"),
    ));
    triples.push(triple("fInside", geo_relation("sfWithin"), node("fSquare")));
    triples.push(triple("gOutside", geo_relation("rcc8ec"), node("gSquare")));
    triples
}

/// Freeze `triples` into a dataset, interning in exactly the order given.
///
/// The order is a parameter because the determinism test builds the same triples
/// twice, in opposite orders, and needs two datasets whose internal id spaces
/// genuinely differ.
fn dataset_of(triples: &[Triple]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for entry in triples {
        let subject = builder.intern_iri(&entry.subject);
        let predicate = builder.intern_iri(&entry.predicate);
        let object = match &entry.object {
            Object::Iri(iri) => builder.intern_iri(iri),
            Object::Literal { lexical, datatype } => {
                builder.intern_literal(RdfLiteral::typed(lexical, datatype))
            }
        };
        builder.push_quad(subject, predicate, object, None);
    }
    builder
        .freeze()
        .expect("the fixture is a well-formed dataset")
}

/// The host's coordinate reference system.
fn crs() -> Crs {
    Crs::new(CRS).expect("a non-empty IRI")
}

/// The host's vocabulary.
fn vocab() -> GeoVocab {
    GeoVocabBuilder::new(GEO, GEOF, crs(), crs())
        .expect("both namespaces are non-empty")
        .build()
}

/// The conformance class in play: `geo:asWKT` alone, over every graph.
fn config() -> GeoIndexConfig {
    GeoIndexConfig::new(
        vec![TermValue::iri(geo(GeoTerm::AsWkt))],
        GraphSelector::Any,
    )
    .expect("one distinct serialization property IRI")
}

/// Project `dataset` and register the Simple Features family over it.
fn registry_over(dataset: &RdfDataset) -> PropertyFunctionRegistry {
    let index = Arc::new(
        GeoIndex::from_dataset(dataset, &vocab(), &config())
            .expect("every fixture geometry is well-formed WKT"),
    );
    let mut registry = PropertyFunctionRegistry::new();
    register(
        &mut registry,
        &vocab(),
        &index,
        &[RelationFamily::SimpleFeatures],
    )
    .expect("one relation family is not the empty set");
    registry
}

/// The parser configuration a host derives **from the registry it built**: the
/// registry's keys are exact IRIs, and that is exactly what
/// [`ParserOptions::property_fn_iris`] matches on.
fn parser_options(registry: &PropertyFunctionRegistry, extra_iris: &[String]) -> ParserOptions {
    let mut property_fn_iris: Vec<String> = registry
        .describe()
        .expect("no registered relation's declaration methods panic")
        .into_iter()
        .map(|descriptor| descriptor.iri)
        .collect();
    property_fn_iris.extend(extra_iris.iter().cloned());
    ParserOptions {
        extension_fn_namespaces: Vec::new(),
        property_fn_namespaces: Vec::new(),
        property_fn_iris,
    }
}

// ---------------------------------------------------------------------------
// Driving the engine
// ---------------------------------------------------------------------------

/// One SELECT answer: the projected variables and the solution rows.
#[derive(Clone, Debug)]
struct Answer {
    /// The projected variable names, in projection order.
    variables: Vec<String>,
    /// The solution rows, in the engine's emission order.
    rows: Vec<Vec<Option<TermValue>>>,
}

/// Wrap `body` in the fixture's PREFIX declarations.
fn query_text(body: &str) -> String {
    format!("PREFIX geo: <{GEO}>\nPREFIX ex: <{EX}>\n{body}\n")
}

/// Parse and evaluate `query` against `dataset` with `registry` in scope.
///
/// Both halves of the seam's configuration are supplied explicitly: the parser
/// options decide whether `geo:sfWithin` is a call or an ordinary triple
/// pattern, and `QueryOptions::property_functions` is the registry the plan is
/// admitted against and evaluated with.
fn run(
    dataset: &RdfDataset,
    registry: &PropertyFunctionRegistry,
    options: ParserOptions,
    query: &str,
) -> Result<Answer, String> {
    let result = NativeSparqlEngine::new()
        .with_parser_options(options)
        .query_with_options_view(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: registry,
                ..QueryOptions::EMPTY
            },
        )
        .map_err(|error| error.to_string())?;
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("a SELECT returns solutions, got {result:?}");
    };
    Ok(Answer { variables, rows })
}

/// [`run`], asserting the query succeeds.
fn answer(
    dataset: &RdfDataset,
    registry: &PropertyFunctionRegistry,
    options: ParserOptions,
    query: &str,
) -> Answer {
    run(dataset, registry, options, query).unwrap_or_else(|error| {
        panic!("the query must evaluate, but was refused: {error}\n{query}")
    })
}

impl Answer {
    /// The column index of `variable`.
    fn column(&self, variable: &str) -> usize {
        self.variables
            .iter()
            .position(|name| name == variable)
            .unwrap_or_else(|| panic!("?{variable} must be projected, got {:?}", self.variables))
    }

    /// The `(left, right)` cells of every row, rendered as fixture local names.
    fn pairs(&self, left: &str, right: &str) -> Vec<(String, String)> {
        let left = self.column(left);
        let right = self.column(right);
        self.rows
            .iter()
            .map(|row| (render(row[left].as_ref()), render(row[right].as_ref())))
            .collect()
    }

    /// One column's cells, rendered.
    fn column_values(&self, variable: &str) -> Vec<String> {
        let at = self.column(variable);
        self.rows
            .iter()
            .map(|row| render(row[at].as_ref()))
            .collect()
    }
}

/// Render a solution cell: a fixture IRI as its local name, a literal as its
/// lexical form. An unbound cell is rendered as `UNBOUND` rather than skipped,
/// so a missing binding cannot be mistaken for a missing row.
fn render(cell: Option<&TermValue>) -> String {
    match cell {
        Some(TermValue::Iri(iri)) => iri.strip_prefix(EX).unwrap_or(iri).to_owned(),
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        Some(other) => format!("{other:?}"),
        None => "UNBOUND".to_owned(),
    }
}

/// A `(left, right)` pair, for readability in the expected tables.
fn pair(left: &str, right: &str) -> (String, String) {
    (left.to_owned(), right.to_owned())
}

/// The complete, exact `geo:sfWithin` answer over [`map_triples`].
///
/// Seven spatial objects are indexed — three features, three of their default
/// geometries, and the bare geometry — and `sfWithin` is reflexive, so each
/// object is within itself, within its twin (the feature and its default
/// geometry denote the same geometry), and within every object carrying the
/// square. Nineteen rows, in the `(?so1, ?so2)` ascending order the relation
/// contracts to.
fn expected_within() -> Vec<(String, String)> {
    vec![
        pair("fInside", "fInside"),
        pair("fInside", "fSquare"),
        pair("fInside", "gInside"),
        pair("fInside", "gSquare"),
        pair("fOutside", "fOutside"),
        pair("fOutside", "gOutside"),
        pair("fSquare", "fSquare"),
        pair("fSquare", "gSquare"),
        pair("gBare", "fSquare"),
        pair("gBare", "gBare"),
        pair("gBare", "gSquare"),
        pair("gInside", "fInside"),
        pair("gInside", "fSquare"),
        pair("gInside", "gInside"),
        pair("gInside", "gSquare"),
        pair("gOutside", "fOutside"),
        pair("gOutside", "gOutside"),
        pair("gSquare", "fSquare"),
        pair("gSquare", "gSquare"),
    ]
}

/// The `?f geo:sfWithin ?g` query text, spelled once so every test below drives
/// the identical string.
fn within_query() -> String {
    query_text("SELECT ?f ?g WHERE { ?f geo:sfWithin ?g }")
}

// ---------------------------------------------------------------------------
// 1 — the headline acceptance test
// ---------------------------------------------------------------------------

/// `?f geo:sfWithin ?g`, written as SPARQL text, returns EXACTLY the nineteen
/// rows the Clause 13 rule entails over the fixture map — not a superset, and
/// above all not a short bag.
///
/// The assertion is `assert_eq!` on a sorted `Vec`, never a count and never a
/// lower bound: a relation that indexed only features would answer a perfectly
/// well-formed *seven*-row bag that the engine would read as complete, and a
/// `rows.len() >= 4` style assertion would pass for it.
#[test]
fn the_rewritten_within_predicate_answers_exactly_the_entailed_rows_from_query_text() {
    let dataset = dataset_of(&map_triples());
    let registry = registry_over(&dataset);
    let answered = answer(
        &dataset,
        &registry,
        parser_options(&registry, &[]),
        &within_query(),
    );

    let mut rows = answered.pairs("f", "g");
    rows.sort();
    assert_eq!(
        rows,
        expected_within(),
        "the query text must answer with exactly the entailed geo:sfWithin pairs; a short bag \
         here is a wrong answer the engine cannot detect"
    );
}

/// All four RIF branches of the Clause 13 rule are reachable **from query
/// text**, each named in its own assertion.
///
/// This is the substance of the acceptance criterion. The rule is a disjunction
/// of four `And`s because either side of a spatial relation may be a
/// `geo:Feature` or a `geo:Geometry`, and a rewrite that dereferences only
/// features drops three of the four branches while still answering rows — the
/// classic short bag. Naming each branch separately means a failure says which
/// branch went missing rather than merely that a count moved.
#[test]
fn all_four_rewrite_branches_are_reachable_from_query_text() {
    let dataset = dataset_of(&map_triples());
    let registry = registry_over(&dataset);
    let rows = answer(
        &dataset,
        &registry,
        parser_options(&registry, &[]),
        &within_query(),
    )
    .pairs("f", "g");

    assert!(
        rows.contains(&pair("fInside", "fSquare")),
        "branch 1 (feature/feature): ?so1 and ?so2 both dereference geo:hasDefaultGeometry — \
         missing from {rows:?}"
    );
    assert!(
        rows.contains(&pair("fInside", "gSquare")),
        "branch 2 (feature/geometry): ?so1 dereferences, ?so2 carries geo:asWKT itself — \
         missing from {rows:?}"
    );
    assert!(
        rows.contains(&pair("gInside", "fSquare")),
        "branch 3 (geometry/feature): ?so1 carries geo:asWKT itself, ?so2 dereferences — \
         missing from {rows:?}"
    );
    assert!(
        rows.contains(&pair("gBare", "gSquare")),
        "branch 4 (geometry/geometry): neither side is a feature, and ?so1 is a bare \
         geo:Geometry with no feature above it at all — missing from {rows:?}"
    );

    // The four `contains` assertions above name the branches, and a bag holding
    // ONLY those four rows satisfies every one of them. Naming a branch is a
    // claim about reachability, not about completeness, so the bag is closed here
    // too: the four branches are not merely present, they are the whole answer.
    let mut all = rows;
    all.sort();
    assert_eq!(
        all,
        expected_within(),
        "the four named branches must be reachable AND be the entire bag; four `contains` \
         assertions alone are satisfied by a fifteen-row short bag"
    );
}

// ---------------------------------------------------------------------------
// 2 — the rewritten predicate composes with ordinary data
// ---------------------------------------------------------------------------

/// The relation's rows join back through the subject into an ordinary basic
/// graph pattern.
///
/// A property function that answered in isolation but whose subject bindings did
/// not join against the graph would be useless: every real GeoSPARQL query
/// filters spatially and then reads attributes. The exact multiset is asserted —
/// four rows for the inside feature (one per `?g` it is within), two each for the
/// outside and square features, and nothing at all for the four geometry nodes,
/// which carry no `ex:name`.
#[test]
fn the_rewritten_predicate_joins_back_through_the_subject_into_a_basic_graph_pattern() {
    let dataset = dataset_of(&map_triples());
    let registry = registry_over(&dataset);
    let answered = answer(
        &dataset,
        &registry,
        parser_options(&registry, &[]),
        &query_text("SELECT ?name WHERE { ?f geo:sfWithin ?g . ?f ex:name ?name }"),
    );

    let mut names = answered.column_values("name");
    names.sort();
    assert_eq!(
        names,
        vec![
            "inside".to_owned(),
            "inside".to_owned(),
            "inside".to_owned(),
            "inside".to_owned(),
            "outside".to_owned(),
            "outside".to_owned(),
            "square".to_owned(),
            "square".to_owned(),
        ],
        "each relation row must join to its subject's ex:name, and the geometry nodes — which \
         have no name — must contribute nothing"
    );

    // The bound-object shape as well, so the join is exercised in both the
    // free-object and bound-object modes the relation declares.
    let mut bounded = answer(
        &dataset,
        &registry,
        parser_options(&registry, &[]),
        &query_text("SELECT ?name WHERE { ?f geo:sfWithin ex:fSquare . ?f ex:name ?name }"),
    )
    .column_values("name");
    bounded.sort();
    assert_eq!(
        bounded,
        vec!["inside".to_owned(), "square".to_owned()],
        "with the object bound the relation must restrict to that object's rows and still join"
    );
}

/// `geo:sfContains` is the converse of `geo:sfWithin` over the same data, with
/// both directions driven through query text.
///
/// Asserting the transpose rather than a hand-written second table is the point:
/// a hand-written table can be wrong in the same direction as the code. If the
/// two relations shared an implementation that ignored argument order, the
/// transpose assertion would fail while a symmetric expectation would pass.
#[test]
fn contains_is_the_converse_of_within_over_the_same_data() {
    let dataset = dataset_of(&map_triples());
    let registry = registry_over(&dataset);
    let options = parser_options(&registry, &[]);

    let mut within = answer(&dataset, &registry, options.clone(), &within_query()).pairs("f", "g");
    within.sort();
    let mut contains = answer(
        &dataset,
        &registry,
        options,
        &query_text("SELECT ?f ?g WHERE { ?f geo:sfContains ?g }"),
    )
    .pairs("f", "g");
    contains.sort();

    // The transpose below is derived from the OBSERVED `within` bag, so on its own
    // it is a purely relative claim. `register` hands the SAME `Arc<GeoIndex>` to
    // every relation, so an index-level short bag drops rows from `sfWithin` and
    // `sfContains` in perfectly transposed correlation and the equality still
    // holds. Both sides are therefore anchored absolutely first — otherwise
    // `geo:sfContains` would have no committed row list anywhere in this suite.
    assert_eq!(
        within,
        expected_within(),
        "the converse claim is only worth making over a `within` bag that is itself complete"
    );

    let mut transposed: Vec<(String, String)> = within
        .iter()
        .map(|(left, right)| (right.clone(), left.clone()))
        .collect();
    transposed.sort();
    assert_eq!(
        contains, transposed,
        "geo:sfContains must be exactly geo:sfWithin with the arguments exchanged"
    );

    let mut expected_contains: Vec<(String, String)> = expected_within()
        .into_iter()
        .map(|(left, right)| (right, left))
        .collect();
    expected_contains.sort();
    assert_eq!(
        contains, expected_contains,
        "and geo:sfContains answers exactly that committed row list, not merely a bag that \
         happens to transpose whatever geo:sfWithin returned"
    );

    // And the ordered pair that makes the claim about argument order rather than
    // about the two relations never disagreeing.
    assert!(
        contains.contains(&pair("gSquare", "fInside")),
        "the square contains the inside feature: {contains:?}"
    );
    assert!(
        !within.contains(&pair("gSquare", "fInside")),
        "and the square is NOT within it: {within:?}"
    );
}

// ---------------------------------------------------------------------------
// 3 — asserted triples are entailed too
// ---------------------------------------------------------------------------

/// An asserted `geo:sfWithin` triple in the data matches alongside the computed
/// rows, and a pair that is both asserted and computed appears exactly once.
///
/// `:-` in Clause 13 is an entailment rule, not a definition. Dropping asserted
/// triples would lose rows a plain BGP would have found; emitting a
/// both-asserted-and-computed pair twice would produce a duplicate solution that
/// no query text explains, because BGP matching over a *set* of triples yields
/// one solution.
#[test]
fn an_asserted_within_triple_matches_alongside_the_computed_rows_and_is_not_duplicated() {
    let dataset = dataset_of(&asserted_triples());
    let registry = registry_over(&dataset);
    let mut rows = answer(
        &dataset,
        &registry,
        parser_options(&registry, &[]),
        &within_query(),
    )
    .pairs("f", "g");
    rows.sort();

    let mut expected = expected_within();
    // `ex:fOutside geo:sfWithin ex:fSquare` is refuted by the geometry — the
    // outside point is nowhere near the square — and matches purely because it
    // is asserted.
    expected.push(pair("fOutside", "fSquare"));
    expected.sort();
    assert_eq!(
        rows, expected,
        "the asserted pair must be added to the computed ones, and nothing else may change"
    );

    let duplicated = rows
        .iter()
        .filter(|candidate| **candidate == pair("fInside", "fSquare"))
        .count();
    assert_eq!(
        duplicated, 1,
        "ex:fInside geo:sfWithin ex:fSquare is BOTH asserted and computed, and one entailed \
         triple is one solution: {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// 4 — LIMIT returns the prefix, not a different bag
// ---------------------------------------------------------------------------

/// `LIMIT k` returns exactly the first `k` rows of the unlimited answer, for
/// every `k` from zero through two past the answer's length.
///
/// The relation is handed a row ceiling by the engine when a limit is in play,
/// and a cursor that spent that ceiling on rows it then filtered out would hand
/// back fewer usable rows than the engine asked for — which the engine reads as
/// exhaustion. The failure is a *short bag that looks complete*, so the check has
/// to be that the limited answer is the exact prefix, not merely that it has at
/// most `k` rows.
#[test]
fn limit_k_returns_exactly_the_prefix_of_the_unlimited_answer() {
    let dataset = dataset_of(&map_triples());
    let registry = registry_over(&dataset);
    let options = parser_options(&registry, &[]);

    // Deliberately NOT sorted: the prefix claim is about the order the engine
    // actually emits, which is the order a limit truncates.
    let unlimited = answer(&dataset, &registry, options.clone(), &within_query()).pairs("f", "g");
    // Content, not merely length: a bag that dropped one entailed row and gained
    // one spurious row has the right length. `expected_within()` is written in the
    // `(?so1, ?so2)` ascending order the relation contracts to, so comparing the
    // UNSORTED answer against it pins the emission order end to end — the claim
    // every other test in this file erases by sorting first, and the claim a
    // prefix assertion is meaningless without.
    assert_eq!(
        unlimited,
        expected_within(),
        "the unlimited answer is the full entailed bag, in the contracted emission order"
    );

    for k in 0..=unlimited.len() + 2 {
        let limited = answer(
            &dataset,
            &registry,
            options.clone(),
            &query_text(&format!(
                "SELECT ?f ?g WHERE {{ ?f geo:sfWithin ?g }} LIMIT {k}"
            )),
        )
        .pairs("f", "g");
        let expected = &unlimited[..k.min(unlimited.len())];
        assert_eq!(
            limited, expected,
            "LIMIT {k} must return the first {k} rows of the unlimited answer, in the same order"
        );
    }
}

// ---------------------------------------------------------------------------
// 5 — the two ways an IRI can fail to be a call
// ---------------------------------------------------------------------------

/// An IRI declared in [`ParserOptions::property_fn_iris`] but absent from the
/// registry is a **hard error**, and the neighbouring registered IRI still
/// answers under the identical configuration.
///
/// The refusal is the right answer because the host declared that IRI to be the
/// seam's: reading it as ordinary data instead would answer the empty bag for a
/// query the host believes is spatial. The valid neighbour is what stops that
/// refusal from being over-broad — a configuration that refused *every* geo
/// predicate would pass the first half of this test and be useless.
#[test]
fn a_declared_but_unregistered_relation_iri_is_refused_while_its_registered_neighbour_answers() {
    let dataset = dataset_of(&map_triples());
    let registry = registry_over(&dataset);
    // RCC8 is not a registered family here, so `geo:rcc8ec` is declared to the
    // parser and resolvable by nothing.
    let unregistered = geo_relation("rcc8ec");
    let options = parser_options(&registry, std::slice::from_ref(&unregistered));

    let error = run(
        &dataset,
        &registry,
        options.clone(),
        &query_text("SELECT ?f ?g WHERE { ?f geo:rcc8ec ?g }"),
    )
    .expect_err("a declared IRI with nothing to resolve against must not read the graph instead");
    assert!(
        error.contains("property function"),
        "the refusal must name the seam that could not resolve the IRI, got {error}"
    );

    // The neighbouring VALID case, under the IDENTICAL parser options: the
    // registered relation still answers its full bag.
    let mut rows = answer(&dataset, &registry, options, &within_query()).pairs("f", "g");
    rows.sort();
    assert_eq!(
        rows,
        expected_within(),
        "declaring an unregistered sibling IRI must not disturb the registered relation"
    );
}

/// An IRI in neither the parser configuration nor the registry is an **ordinary
/// triple pattern**, matching the asserted statements in the graph and nothing
/// else — and the registered relation in the same run is still a call.
///
/// This is the other half of the seam being configuration: there is no default
/// that turns a `geo:` predicate into a rewrite. A `geo:rcc8ec` statement written
/// in the data is data, and the query that reads it must see exactly that one
/// statement rather than a computed RCC8 answer.
#[test]
fn an_undeclared_unregistered_geo_iri_is_an_ordinary_triple_pattern_matching_the_asserted_data() {
    let dataset = dataset_of(&asserted_triples());
    let registry = registry_over(&dataset);
    let options = parser_options(&registry, &[]);

    let rows = answer(
        &dataset,
        &registry,
        options.clone(),
        &query_text("SELECT ?s ?o WHERE { ?s geo:rcc8ec ?o }"),
    )
    .pairs("s", "o");
    assert_eq!(
        rows,
        vec![pair("gOutside", "gSquare")],
        "an unconfigured, unregistered geo: predicate reads the graph: exactly the one asserted \
         statement, never a computed RCC8 bag"
    );

    // The neighbouring VALID case in the same configuration: the registered
    // relation is still rewritten, so the ordinary reading above is about THIS
    // IRI rather than about the seam being off.
    let mut within = answer(&dataset, &registry, options, &within_query()).pairs("f", "g");
    within.sort();
    let mut expected = expected_within();
    expected.push(pair("fOutside", "fSquare"));
    expected.sort();
    assert_eq!(
        within, expected,
        "geo:sfWithin is still a rewritten call in the very same run"
    );
}

// ---------------------------------------------------------------------------
// 6 — determinism at the consumer-visible artefact
// ---------------------------------------------------------------------------

/// Serialize an [`Answer`] as SPARQL 1.1 Query Results JSON.
///
/// Hand-written rather than routed through `purrdf-sparql-results`: that crate is
/// **not** among `purrdf-geo`'s `[dev-dependencies]`, and adding one would edit a
/// manifest this file is not permitted to touch. What the determinism claim needs
/// is a *consumer-visible artefact* — a byte string a host would ship — and this
/// renderer is one: it walks the answer in emission order, writes no map's
/// iteration order, and interpolates no dataset-local id. If two runs disagree
/// anywhere in the answer, these bytes disagree.
fn to_results_json(answer: &Answer) -> String {
    let mut out = String::from("{\"head\":{\"vars\":[");
    for (at, variable) in answer.variables.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&escape_json(variable));
        out.push('"');
    }
    out.push_str("]},\"results\":{\"bindings\":[");
    for (at, row) in answer.rows.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push('{');
        let mut written = 0_usize;
        for (variable, cell) in answer.variables.iter().zip(row.iter()) {
            let Some(term) = cell.as_ref() else { continue };
            if written > 0 {
                out.push(',');
            }
            written += 1;
            out.push('"');
            out.push_str(&escape_json(variable));
            out.push_str("\":");
            out.push_str(&binding_json(term));
        }
        out.push('}');
    }
    out.push_str("]}}");
    out
}

/// One bound term, as a JSON results binding object.
fn binding_json(term: &TermValue) -> String {
    match term {
        TermValue::Iri(iri) => format!("{{\"type\":\"uri\",\"value\":\"{}\"}}", escape_json(iri)),
        TermValue::Literal {
            lexical_form,
            datatype,
            ..
        } => format!(
            "{{\"type\":\"literal\",\"value\":\"{}\",\"datatype\":\"{}\"}}",
            escape_json(lexical_form),
            escape_json(datatype)
        ),
        TermValue::Blank { label, .. } => {
            format!(
                "{{\"type\":\"bnode\",\"value\":\"{}\"}}",
                escape_json(label)
            )
        }
        TermValue::Triple { .. } => {
            panic!("no fixture query projects a triple term, so none can reach the serializer")
        }
    }
}

/// The two escapes the fixture terms can possibly need.
fn escape_json(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The consumer-visible artefact is byte-identical across two runs, and across
/// two datasets that hold the same triples but interned them in **opposite
/// orders**.
///
/// Three things have to be true for this test to mean anything, and all three
/// are asserted rather than assumed:
///
/// 1. The two datasets genuinely differ — a probe term resolves to a different
///    dataset-local id in each. Without this, "determinism survived" would be a
///    statement about one dataset compared with itself.
/// 2. The JSON is non-vacuous — it carries an IRI the answer must contain. Two
///    empty answers are trivially byte-identical, and an empty-bag regression is
///    exactly the failure this seam is prone to.
/// 3. Only then, the bytes are equal.
#[test]
fn the_serialized_answer_is_byte_identical_across_runs_and_across_ingestion_orders() {
    let forward_triples = map_triples();
    let mut reversed_triples = map_triples();
    reversed_triples.reverse();

    let forward = dataset_of(&forward_triples);
    let reversed = dataset_of(&reversed_triples);

    // 1. The two datasets are genuinely different objects, not one dataset
    //    compared with itself.
    let probe = TermValue::iri(ex("gSquare"));
    assert_ne!(
        forward.term_id_by_value(&probe),
        reversed.term_id_by_value(&probe),
        "the two ingestion orders must mint different dataset-local ids for the probe term, or \
         there is nothing for determinism to survive"
    );

    let forward_registry = registry_over(&forward);
    let reversed_registry = registry_over(&reversed);

    let render_answer = |dataset: &RdfDataset, registry: &PropertyFunctionRegistry| {
        to_results_json(&answer(
            dataset,
            registry,
            parser_options(registry, &[]),
            &within_query(),
        ))
    };

    let first = render_answer(&forward, &forward_registry);
    let second = render_answer(&forward, &forward_registry);
    let other_order = render_answer(&reversed, &reversed_registry);

    // 2. Non-vacuous: an empty answer would serialize to an empty bindings array
    //    and pass the equality below for the wrong reason.
    assert!(
        first.contains(&ex("gSquare")),
        "the serialized answer must carry the fixture's IRIs, got {first}"
    );
    assert!(
        first.contains("\"bindings\":[{"),
        "the serialized answer must hold at least one binding, got {first}"
    );
    // "At least one binding" is satisfied by a bag short by eighteen of nineteen
    // rows — and such a bag would be short IDENTICALLY in both runs and both
    // intern orders, so the byte equality below would still hold. The count is
    // therefore pinned, not merely its non-zeroness.
    assert_eq!(
        first.matches("\"f\":").count(),
        expected_within().len(),
        "the serialized answer must carry every entailed row, got {first}"
    );

    // 3. And the bytes agree.
    assert_eq!(
        first, second,
        "two runs over the same dataset must serialize byte-identically"
    );
    assert_eq!(
        first, other_order,
        "two datasets holding the same triples in opposite intern orders must serialize \
         byte-identically: no dataset-local id may reach a result"
    );
}
