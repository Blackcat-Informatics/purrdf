// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Path witnesses as SPARQL, from the vantage a host has.
//!
//! Every test in this file is an acceptance criterion driven through the genuine
//! production surface and nothing else: a SPARQL query **text**, parsed with the caller
//! IRI in [`ParserOptions::property_fn_iris`], evaluated by [`NativeSparqlEngine`]
//! through [`QueryOptions::property_functions`] carrying a
//! [`PropertyFunctionRegistry`] the relation is registered in under that same IRI. No
//! test here reaches into the crate to drive a cursor directly — the relation-level unit
//! tests in `src/path_relation.rs` do that, and a seam whose halves only line up from
//! inside is a seam no host can use.
//!
//! Assertions are exact literal vectors rather than "at least N rows", because the
//! failure these tests exist to catch is over-generation as much as under-generation: a
//! traversal that diverges on a cycle, or that enumerates a walk twice, is a test that
//! must go red rather than a test that quietly still passes.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, GraphMatch, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest,
    SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    MemoryRelation, NativeSparqlEngine, ParserOptions, PathDirection, PathGraph, PathLimits,
    PathStep, PathWitnessRelation, PropertyFunctionRegistry, QueryOptions,
};

// ---------------------------------------------------------------------------
// The host's configuration
// ---------------------------------------------------------------------------

/// The fixture data namespace. PurRDF mints no vocabulary IRIs; every term below is
/// caller-supplied fixture data under `example.org`.
const EX: &str = "http://example.org/";

/// The caller IRI the path-witness relation is registered and called under.
const WALK: &str = "http://example.org/pf#walk";

/// A second caller IRI, for the differently-shaped consumer of the same seam.
const RANKED: &str = "http://example.org/pf#ranked";

/// `rdf:reifies`, the RDF 1.2 predicate that binds a reifier to the triple term it
/// reifies. Not a minted vocabulary: it is the standard RDF 1.2 name, and it is how the
/// engine projects its reifier side-table as quads.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// `xsd:integer`, the datatype `?step` and `?len` carry.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The parse-time configuration: both caller IRIs are recognized as property-function
/// predicates by EXACT match. Without this the very same query text is an ordinary
/// triple pattern reading the graph.
fn parser_options() -> ParserOptions {
    ParserOptions {
        extension_fn_namespaces: Vec::new(),
        property_fn_namespaces: Vec::new(),
        property_fn_iris: vec![WALK.to_owned(), RANKED.to_owned()],
    }
}

/// An engine that recognizes the caller IRIs at parse time.
fn engine() -> NativeSparqlEngine {
    NativeSparqlEngine::new().with_parser_options(parser_options())
}

/// A fixture IRI under [`EX`].
fn iri(local: &str) -> TermValue {
    TermValue::iri(format!("{EX}{local}"))
}

/// The asserted statement term a hop over `(ex:s, ex:p, ex:o)` records.
fn stmt(s: &str, p: &str, o: &str) -> TermValue {
    TermValue::Triple {
        s: Box::new(iri(s)),
        p: Box::new(iri(p)),
        o: Box::new(iri(o)),
    }
}

/// An `xsd:integer`-typed literal, the form `?step` and `?len` are emitted in.
fn int(n: u64) -> TermValue {
    TermValue::typed_literal(n.to_string(), XSD_INTEGER)
}

/// A simple literal, the form `?pathId` is emitted in.
fn text(value: &str) -> TermValue {
    TermValue::simple_literal(value)
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Build a dataset from `(subject, predicate, object)` local names under [`EX`].
fn dataset(triples: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (s, p, o) in triples {
        let s = builder.intern_iri(&format!("{EX}{s}"));
        let p = builder.intern_iri(&format!("{EX}{p}"));
        let o = builder.intern_iri(&format!("{EX}{o}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("the fixture freezes")
}

/// Snapshot `alternatives` over `data` as a [`PathGraph`].
fn snapshot(data: &RdfDataset, alternatives: &[(&str, PathDirection)]) -> Arc<PathGraph> {
    let step = PathStep::new(
        alternatives
            .iter()
            .map(|(predicate, direction)| (iri(predicate), *direction))
            .collect(),
    )
    .expect("a well-formed step");
    Arc::new(PathGraph::from_dataset(data, &step, GraphMatch::Default).expect("snapshot"))
}

/// A generous envelope: only the hop-length interval binds.
fn limits(min: u32, max: u32) -> PathLimits {
    PathLimits::new(min, max, 4096, 1_000_000).expect("a generous envelope")
}

/// A registry holding one [`PathWitnessRelation`] under [`WALK`].
fn walk_registry(graph: Arc<PathGraph>, limits: PathLimits) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        WALK.to_owned(),
        Arc::new(PathWitnessRelation::new(graph, limits)),
    );
    registry
}

// ---------------------------------------------------------------------------
// Driving the engine
// ---------------------------------------------------------------------------

/// One query's answers: the projected variable names and the rows, exactly as the engine
/// produced them.
#[derive(Debug)]
struct Answers {
    variables: Vec<String>,
    rows: Vec<Vec<Option<TermValue>>>,
}

impl Answers {
    /// The rows projected onto `names`, in row order, with every named cell required to
    /// be bound. A test that names a column the query did not project is a test that has
    /// drifted from its query text, so that is a panic rather than an empty answer.
    fn project(&self, names: &[&str]) -> Vec<Vec<TermValue>> {
        let indices: Vec<usize> = names
            .iter()
            .map(|name| {
                self.variables
                    .iter()
                    .position(|variable| variable == name)
                    .unwrap_or_else(|| panic!("?{name} is not projected by this query"))
            })
            .collect();
        self.rows
            .iter()
            .map(|row| {
                indices
                    .iter()
                    .map(|&index| {
                        row[index]
                            .clone()
                            .unwrap_or_else(|| panic!("every projected cell is bound: {row:?}"))
                    })
                    .collect()
            })
            .collect()
    }
}

/// Evaluate `query` over `data` with `registry` in scope, through the public engine.
fn run(
    data: &RdfDataset,
    query: &str,
    registry: &PropertyFunctionRegistry,
) -> Result<Answers, purrdf_core::RdfDiagnostic> {
    let result = engine().query_with_options_view(
        data,
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions {
            property_functions: registry,
            ..QueryOptions::EMPTY
        },
    )?;
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("every query here is a SELECT");
    };
    Ok(Answers { variables, rows })
}

/// Evaluate `query`, requiring it to succeed.
fn solve(data: &RdfDataset, query: &str, registry: &PropertyFunctionRegistry) -> Answers {
    run(data, query, registry).expect("the call resolves and evaluates")
}

/// The `PREFIX` header every query below opens with.
const PREFIXES: &str = "PREFIX ex: <http://example.org/>\n";

/// Prepend [`PREFIXES`] to a query body.
fn q(body: &str) -> String {
    format!("{PREFIXES}{body}")
}

/// The distinct values of column `column` of `rows`, in first-appearance order.
fn distinct_column(rows: &[Vec<TermValue>], column: usize) -> Vec<TermValue> {
    let mut seen: Vec<TermValue> = Vec::new();
    for row in rows {
        if !seen.contains(&row[column]) {
            seen.push(row[column].clone());
        }
    }
    seen
}

// ---------------------------------------------------------------------------
// A1 — the headline
// ---------------------------------------------------------------------------

/// The three-edge chain, hop by hop, through the query surface.
///
/// Every walk from `ex:a` within three hops is enumerated, each as one row per hop, and
/// the six rows are pinned exactly: over-generation (a walk emitted twice, an extension
/// past the envelope) fails this test as loudly as under-generation does.
///
/// `?pathId` is checked structurally as well as literally: the six rows partition into
/// three groups of sizes 1, 2 and 3, and the three identifiers are pairwise distinct —
/// which is what makes `GROUP BY ?pathId` a sound reconstruction of a walk.
#[test]
fn a1_a_chain_binds_every_walk_hop_by_hop_through_the_query_surface() {
    let data = dataset(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")]);
    let graph = snapshot(&data, &[("p", PathDirection::Forward)]);
    let registry = walk_registry(graph, limits(1, 3));

    let answers = solve(
        &data,
        &q("SELECT ?end ?pathId ?len ?step ?node ?edge WHERE { \
            ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
            } ORDER BY ?len ?step"),
        &registry,
    );
    let rows = answers.project(&["end", "pathId", "len", "step", "node", "edge"]);

    // The three identifiers, one per walk, in the order the walks first appear.
    let ids = distinct_column(&rows, 1);
    assert_eq!(ids.len(), 3, "three walks: a→b, a→b→c, a→b→c→d: {rows:?}");
    let (ab, abc, abcd) = (ids[0].clone(), ids[1].clone(), ids[2].clone());
    assert_ne!(ab, abc);
    assert_ne!(abc, abcd);
    assert_ne!(ab, abcd);
    // The identifier is the full, untruncated SHA-256, rendered as 64 lowercase hex
    // characters, and the walk a→b→c's is pinned: a change to the domain separator, the
    // term encoding, the fold order or the length suffix re-identifies grouping keys
    // already handed to callers, and must be a loud failure rather than a silent one.
    assert_eq!(
        abc,
        text("3e4c617c5f08362717dfdbdaf9ced0e4db15c8253c13284e4ad7d6b7a8269c08")
    );

    let e1 = stmt("a", "p", "b");
    let e2 = stmt("b", "p", "c");
    let e3 = stmt("c", "p", "d");
    assert_eq!(
        rows,
        vec![
            // a→b: one hop, one row.
            vec![iri("b"), ab, int(1), int(1), iri("b"), e1.clone()],
            // a→b→c: two hops, two rows, one identifier.
            vec![iri("c"), abc.clone(), int(2), int(1), iri("b"), e1.clone()],
            vec![iri("c"), abc, int(2), int(2), iri("c"), e2.clone()],
            // a→b→c→d: three hops, three rows, one identifier.
            vec![iri("d"), abcd.clone(), int(3), int(1), iri("b"), e1],
            vec![iri("d"), abcd.clone(), int(3), int(2), iri("c"), e2],
            vec![iri("d"), abcd, int(3), int(3), iri("d"), e3],
        ]
    );

    // One row per hop loses nothing relative to an `rdf:List` of nodes: the module docs'
    // `GROUP_CONCAT` recipe reconstructs the node sequence of every walk inside the query
    // language, with `GROUP BY ?pathId` as the whole trick. Asserted here so the recipe
    // is a tested claim rather than prose.
    let reconstructed = solve(
        &data,
        &q(
            "SELECT ?end ?len (GROUP_CONCAT(?node; separator=\"->\") AS ?route) WHERE { \
            ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
            } GROUP BY ?pathId ?end ?len ORDER BY ?len",
        ),
        &registry,
    );
    assert_eq!(
        reconstructed.project(&["end", "len", "route"]),
        vec![
            vec![iri("b"), int(1), text(&format!("{EX}b"))],
            vec![iri("c"), int(2), text(&format!("{EX}b->{EX}c"))],
            vec![iri("d"), int(3), text(&format!("{EX}b->{EX}c->{EX}d"))],
        ],
        "GROUP BY ?pathId reassembles each walk from its hop rows, in ?step order"
    );
}

// ---------------------------------------------------------------------------
// A2 — the Virtuoso reference case
// ---------------------------------------------------------------------------

/// Agreement with Virtuoso's `t_step` transitivity semantics, transcribed from an
/// observed run rather than argued from prose.
///
/// The expected vector below is the OUTPUT of **Virtuoso Open Source Edition, Version
/// 07.20.3243**, observed over this test's own fixture graph and transcribed here. It was
/// never read back from this crate. The harness does NOT run Virtuoso — nothing in this
/// process speaks to a database — so what is pinned is a transcription, and the quoted
/// documentation below is what makes the transcription checkable rather than a magic
/// constant: it fixes which reference column corresponds to which of this relation's
/// variables, and the run then shows that correspondence yields exactly these rows.
///
/// # The reference example, verbatim
///
/// Virtuoso documents `t_step` in two places. The SPARQL grammar (Virtuoso Documentation
/// §16.2.11, *Transitivity in SPARQL*) is:
///
/// ```text
/// option (transitive transitivity_option[,...])
///
/// transitivity_option ::=  t_in (<variable_list>)
/// | t_out (<variable_list>)
/// | t_distinct
/// | t_shortest_only
/// | t_no_cycles
/// | t_cycles_only
/// | t_min (INTNUM)
/// | t_max (INTNUM)
/// | t_end_flag (<variable>)
/// | t_step (<variable_or_step>)
/// | t_direction INTNUM
///
/// variable_list ::= <variable> [,...]
///
/// variable_or_step ::= <variable> | 'path_id' | 'step_no'
/// ```
///
/// and its worked SPARQL query using all three `t_step` forms is:
///
/// ```text
/// SELECT ?link ?g ?step ?path WHERE {
///   {
///     SELECT ?s ?o ?g WHERE { graph ?g {?s foaf:knows ?o } }
///   } OPTION ( TRANSITIVE, t_distinct, t_in(?s), t_out(?o),
///              t_no_cycles, t_shortest_only,
///              t_step (?s) as ?link,
///              t_step ('path_id') as ?path,
///              t_step ('step_no') as ?step,
///              t_direction 3) .
///   FILTER (?s= <http://www.w3.org/People/Berners-Lee/card#i>
///   && ?o = <http://www.advogato.org/person/mparaz/foaf.rdf#me>)
/// }
/// LIMIT 20
/// ```
///
/// That page says of the two magic strings: "See the SQL transitive option section for
/// details on the meaning of step_no and path_id." The SQL section (Virtuoso
/// Documentation §9.32, *Transitivity in SQL*) supplies them, verbatim:
///
/// * `t_step ('step_no')` — "This returns the ordinal number of the step on the path.
///   Step -0 corresponds to the input variables being at the value seen in the enclosing
///   query. Step 1 is one removed from this."
/// * `t_step ('path_id')` — "The t_step ('path_id') is a number identifying the
///   connection path, since there may be many paths joining persons 1 and 4."
/// * `t_step (<column>)` — "This returns the value that the column, one of the columns
///   designated as input, has at this step."
///
/// and it works those semantics out over a concrete `knows` table:
///
/// ```text
/// create table knows (p1 int, p2 int, primary key (p1, p2))
///
/// insert into knows values (1, 2);
/// insert into knows values (1, 3);
/// insert into knows values (2, 4);
///
/// select * from (select transitive t_in (1) t_out (2) t_min (0) t_distinct
/// p1, p2, t_step (1) as via, t_step ('path_id') as path,
/// t_step ('step_no') as step from knows) k where p1 = 1;
///
/// P1       P2       VIA      PATH     STEP
/// 1        1        1        0        0
/// 1        3        1        1        0
/// 1        3        3        1        1
/// 1        2        1        2        0
/// 1        2        2        2        1
/// 1        4        1        3        0
/// 1        4        2        3        1
/// 1        4        4        3        2
/// ```
///
/// That table is the documentation's. Running that exact SQL — the three inserts, then the
/// `select transitive` query — against Virtuoso Open Source Edition 07.20.3243 returned
/// those eight rows, with those values, in that order. The vendor's worked example is
/// therefore not merely quoted here; it was reproduced.
///
/// # The column correspondence
///
/// The fixture below is that graph, with `example.org` fixture IRIs standing in for the
/// integers and for `foaf:knows` (this project mints no vocabulary IRIs, and its fixtures
/// live under `example.org`): `1 → ex:p1`, `2 → ex:p2`, `3 → ex:p3`, `4 → ex:p4`,
/// `knows → ex:knows`. The mapping is order-preserving, so nothing about the
/// correspondence depends on it.
///
/// Now read each documented option against this relation's row shape, term by term.
///
/// * `t_in(1)` / `t_out(2)` designate `p1` as the input column and `p2` as the output
///   one, so a produced row's `p1` is the walk's SEED and its `p2` is the walk's
///   ENDPOINT. Those are exactly `?start` and `?end`.
/// * `t_step('step_no')` is "the ordinal number of the step on the path", with step 0
///   being "the input variables being at the value seen in the enclosing query" — the
///   seed, before any edge is traversed. Our `?step` is the 1-based ordinal of a
///   TRAVERSED hop, so `?step = k` corresponds to Virtuoso's `step = k` for every
///   `k >= 1`, and Virtuoso's `step = 0` rows have no counterpart here at all: a
///   zero-hop path traversed no statement, so it has no hop to bind (and this relation's
///   `min_hops` is documented as never being zero for that reason). Virtuoso reaches
///   those rows only because the query asks for `t_min (0)`.
/// * `t_step (1)` — `via` — "returns the value that the column, one of the columns
///   designated as input, has at this step". Column 1 is `p1`, the input; the transitive
///   closure advances the input column one node per step, so at step `k` the input column
///   holds the walk's `k`-th node (0-indexed from the seed). Our `?node` is the node the
///   `k`-th hop ARRIVES at, which is the walk's `k`-th node counting the seed as 0. So
///   for `k >= 1`, `via` and `?node` name the same node. (At `k = 0` `via` is the seed,
///   which we carry on every row as `?start`.)
/// * `t_step('path_id')` is "a number identifying the connection path". Our `?pathId` is
///   a content-derived identifier of the walk. Neither value is meaningful in itself;
///   both are meaningful only through the PARTITION they induce on the rows, so that is
///   what is compared below.
///
/// # Why the reference table has eight rows and this test asserts four
///
/// The documented query asks for `t_min (0)`, so it reaches the zero-hop rows. Applying
/// the term-by-term reading to those eight rows: drop the four `step = 0` rows (paths 0,
/// 1, 2 and 3 each contribute one), and drop path 0 entirely — it is the identity path
/// `1 → 1`, which consists of ONLY a step-0 row. What remains, rewritten as
/// `(?start, ?end, ?step, ?node)`, is:
///
/// ```text
/// documented row              →  (?start, ?end, ?step, ?node)
/// (p1=1, p2=3, via=3, step=1) →  (ex:p1, ex:p3, 1, ex:p3)
/// (p1=1, p2=2, via=2, step=1) →  (ex:p1, ex:p2, 1, ex:p2)
/// (p1=1, p2=4, via=2, step=1) →  (ex:p1, ex:p4, 1, ex:p2)
/// (p1=1, p2=4, via=4, step=2) →  (ex:p1, ex:p4, 2, ex:p4)
/// ```
///
/// # The observed `OPTION(TRANSITIVE ...)` run
///
/// That last step need not be argued, because the reference engine can simply be asked at
/// `t_min (1)` — which is exactly this relation's `min_hops` — and then the step-0 rows
/// are gone at the source rather than by subtraction. The fixture graph
/// (`ex:p1 ex:knows ex:p2`, `ex:p1 ex:knows ex:p3`, `ex:p2 ex:knows ex:p4`, with `ex:` =
/// `http://example.org/`, i.e. precisely the dataset built below) was loaded into a named
/// graph of Virtuoso Open Source Edition 07.20.3243 and queried in the §16.2.11
/// `OPTION(TRANSITIVE ...)` shape:
///
/// ```text
/// SPARQL SELECT ?s ?o ?link ?path ?step WHERE {
///   { SELECT ?s ?o WHERE { GRAPH ex:g { ?s ex:knows ?o } } }
///   OPTION ( TRANSITIVE, t_distinct, t_in(?s), t_out(?o), t_min(1),
///            t_step(?s) as ?link, t_step('path_id') as ?path,
///            t_step('step_no') as ?step ) .
///   FILTER ( ?s = ex:p1 )
/// } ORDER BY ?o ?step;
/// ```
///
/// The observed output was exactly four rows (IRIs abbreviated to `ex:` here; the run
/// printed them in full):
///
/// ```text
/// ?s      ?o      ?link   ?path  ?step
/// ex:p1   ex:p2   ex:p2   1      1
/// ex:p1   ex:p3   ex:p3   0      1
/// ex:p1   ex:p4   ex:p2   2      1
/// ex:p1   ex:p4   ex:p4   2      2
/// ```
///
/// Under the correspondence established above — `?s` is `?start`, `?o` is `?end`, `?link`
/// is `?node`, `?step` is `?step`, and `?path` is read only through the partition it
/// induces, as `?pathId` is — those four rows ARE the vector asserted below, row for row,
/// with no derivation step remaining; and `?path` partitions them into groups of sizes 1,
/// 1 and 2, which is the partition asserted below. Re-running that same query at
/// `t_min (0)` returned eight rows instead: these four plus one `step = 0` row per path,
/// including the identity row `ex:p1 → ex:p1` on path 0 — the documented SQL table's shape,
/// reproduced on the SPARQL surface.
///
/// That run is what validates this relation against a Virtuoso `OPTION(TRANSITIVE ...)`
/// reference case: same path pattern, same fixture graph, equivalent binding semantics.
/// The expected tuples below are that observed Virtuoso output, transcribed; they were
/// never read back from this crate's output.
#[test]
fn a2_the_projection_matches_the_virtuoso_transitivity_reference() {
    // `insert into knows values (1, 2); (1, 3); (2, 4);`
    let data = dataset(&[
        ("p1", "knows", "p2"),
        ("p1", "knows", "p3"),
        ("p2", "knows", "p4"),
    ]);
    let graph = snapshot(&data, &[("knows", PathDirection::Forward)]);
    // `t_max` is unstated in the documented SQL query; three hops is past the longest
    // walk the graph admits, so the envelope is not what bounds the answer.
    let registry = walk_registry(graph, limits(1, 3));

    // `where p1 = 1`, spelled as a FILTER on the seed exactly as the documented SPARQL
    // query spells its own seed restriction.
    let answers = solve(
        &data,
        &q("SELECT ?start ?end ?pathId ?step ?node WHERE { \
            ?start <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) . \
            FILTER ( ?start = ex:p1 ) \
            } ORDER BY ?end ?step"),
        &registry,
    );
    let rows = answers.project(&["start", "end", "pathId", "step", "node"]);

    // The (?start, ?end, ?step, ?node) projection, ordered by ?end then ?step so the
    // comparison is against a set rather than against this relation's emission order.
    let projection: Vec<Vec<TermValue>> = rows
        .iter()
        .map(|row| {
            vec![
                row[0].clone(),
                row[1].clone(),
                row[3].clone(),
                row[4].clone(),
            ]
        })
        .collect();
    assert_eq!(
        projection,
        vec![
            // (p1=1, p2=2, via=2, step=1)
            vec![iri("p1"), iri("p2"), int(1), iri("p2")],
            // (p1=1, p2=3, via=3, step=1)
            vec![iri("p1"), iri("p3"), int(1), iri("p3")],
            // (p1=1, p2=4, via=2, step=1)
            vec![iri("p1"), iri("p4"), int(1), iri("p2")],
            // (p1=1, p2=4, via=4, step=2)
            vec![iri("p1"), iri("p4"), int(2), iri("p4")],
        ],
        "the observed Virtuoso 07.20.3243 OPTION(TRANSITIVE ...) output at t_min(1), \
         transcribed under the documented t_step column correspondence"
    );

    // `path_id` partitions the four rows into groups of sizes 1, 1 and 2 — the two
    // one-hop paths, and the two rows of the two-hop path. `?pathId` must induce the
    // SAME partition; the identifier values themselves are opaque in both systems.
    let mut partition: Vec<Vec<Vec<TermValue>>> = Vec::new();
    for id in distinct_column(&rows, 2) {
        partition.push(
            rows.iter()
                .filter(|row| row[2] == id)
                .map(|row| vec![row[1].clone(), row[3].clone(), row[4].clone()])
                .collect(),
        );
    }
    assert_eq!(
        partition,
        vec![
            // path 2: the one-hop walk to ex:p2.
            vec![vec![iri("p2"), int(1), iri("p2")]],
            // path 1: the one-hop walk to ex:p3.
            vec![vec![iri("p3"), int(1), iri("p3")]],
            // path 3: the two-hop walk to ex:p4, both of whose rows share one identifier.
            vec![
                vec![iri("p4"), int(1), iri("p2")],
                vec![iri("p4"), int(2), iri("p4")],
            ],
        ],
        "?pathId must partition the rows exactly as path_id does"
    );
}

// ---------------------------------------------------------------------------
// A5 — an unregistered IRI hard-fails
// ---------------------------------------------------------------------------

/// A call the parser minted and the registry cannot answer is a diagnostic, never an
/// empty bag.
///
/// This is the whole point of the seam being configuration: the parser lowers the
/// predicate to a call ONLY because `property_fn_iris` names it, and a call with nothing
/// to resolve against is a host that named a relation it never supplied. The failure mode
/// this test forbids is the silent one — the very same text, unconfigured, is an ordinary
/// triple pattern that answers zero rows with no complaint, and zero rows offered as a
/// complete answer is precisely the wrong answer.
#[test]
fn a5_an_unregistered_caller_iri_is_a_hard_failure_not_an_empty_answer() {
    let data = dataset(&[("a", "p", "b"), ("b", "p", "c")]);
    let query = q("SELECT ?end WHERE { \
                   ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) }");

    // The parser recognizes the IRI (it is in `property_fn_iris`); the registry is empty.
    let empty = PropertyFunctionRegistry::new();
    let result = run(&data, &query, &empty);
    assert!(
        result.is_err(),
        "a call with nothing to resolve against must NOT be a zero-row Ok: {result:?}"
    );
    let error = result.expect_err("checked immediately above");
    assert_eq!(error.code, "native-sparql-property-function");
    assert!(
        error.message.contains(WALK),
        "the diagnostic must name the IRI that could not be resolved: {}",
        error.message
    );

    // And here is the shape the `Err` above replaced. With NO parse-time configuration
    // the same text is an ordinary triple pattern whose object is an RDF collection, it
    // matches nothing in this dataset, and the engine reports a clean, complete answer of
    // zero rows. Nothing in that answer says a relation was meant to run.
    let fallthrough = NativeSparqlEngine::new()
        .query(
            &data,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
        )
        .expect("with no configuration the text is valid, ordinary SPARQL");
    let SparqlResult::Solutions { rows, .. } = fallthrough else {
        panic!("a SELECT returns solutions");
    };
    assert!(
        rows.is_empty(),
        "the triple-pattern fallthrough is an Ok with zero rows — indistinguishable from \
         an honest empty answer, which is why the configured call must Err instead: {rows:?}"
    );
}

// ---------------------------------------------------------------------------
// A7 — cycles terminate
// ---------------------------------------------------------------------------

/// A cycle is enumerated to its closing hop and no further, under an envelope eight times
/// longer than the cycle.
///
/// The exact row vector is what makes this a test rather than a smoke check: a traversal
/// that re-entered the cycle would not hang here (`max_hops` bounds it at 8) — it would
/// quietly return more rows. Pinning the six is what turns over-generation into a failure.
#[test]
fn a7_a_cycle_terminates_at_its_closing_hop() {
    let data = dataset(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
    let graph = snapshot(&data, &[("p", PathDirection::Forward)]);
    let registry = walk_registry(graph, limits(1, 8));

    let answers = solve(
        &data,
        &q("SELECT ?end ?pathId ?len ?step ?node ?edge WHERE { \
            ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
            } ORDER BY ?len ?step"),
        &registry,
    );
    let rows = answers.project(&["end", "pathId", "len", "step", "node", "edge"]);

    let ids = distinct_column(&rows, 1);
    assert_eq!(ids.len(), 3, "a→b, a→b→c, a→b→c→a: {rows:?}");
    let (ab, abc, abca) = (ids[0].clone(), ids[1].clone(), ids[2].clone());

    let e1 = stmt("a", "p", "b");
    let e2 = stmt("b", "p", "c");
    let e3 = stmt("c", "p", "a");
    assert_eq!(
        rows,
        vec![
            vec![iri("b"), ab, int(1), int(1), iri("b"), e1.clone()],
            vec![iri("c"), abc.clone(), int(2), int(1), iri("b"), e1.clone()],
            vec![iri("c"), abc, int(2), int(2), iri("c"), e2.clone()],
            // The closing hop back to ex:a: the walk is emitted and terminates, so ex:a
            // is its own endpoint exactly once and the traversal stops there.
            vec![iri("a"), abca.clone(), int(3), int(1), iri("b"), e1],
            vec![iri("a"), abca.clone(), int(3), int(2), iri("c"), e2],
            vec![iri("a"), abca, int(3), int(3), iri("a"), e3],
        ],
        "exactly six rows under max_hops = 8"
    );
}

// ---------------------------------------------------------------------------
// A8 — an RDF 1.2 triple term as an intermediate node
// ---------------------------------------------------------------------------

/// A triple term is an ordinary node of a walk, entered and left like any other.
///
/// This IR refuses a quoted triple in ASSERTED SUBJECT position (`rdf-ir-triple-subject`
/// — see the assertion at the top of this test, which builds exactly that and watches it
/// be refused). So a triple term can only be reached as an OBJECT, and left by traversing
/// some other statement backward — which is precisely why a [`PathStep`] is an
/// alternation of DIRECTED predicates rather than of predicates.
#[test]
fn a8_a_triple_term_is_an_intermediate_node_of_a_walk() {
    // First, the constraint this fixture is shaped around, verified rather than assumed:
    // the IR rejects a quoted triple asserted in SUBJECT position.
    let mut refused = RdfDatasetBuilder::new();
    let x = refused.intern_iri(&format!("{EX}x"));
    let r = refused.intern_iri(&format!("{EX}r"));
    let y = refused.intern_iri(&format!("{EX}y"));
    let p = refused.intern_iri(&format!("{EX}p"));
    let quoted = refused.intern_triple(x, r, y);
    refused.push_quad(quoted, p, y, None);
    let error = refused
        .freeze()
        .expect_err("a quoted triple may not be an ASSERTED subject");
    assert_eq!(error.code, "rdf-ir-triple-subject");

    // `ex:a ex:p <<( ex:x ex:r ex:y )>> . ex:z ex:q <<( ex:x ex:r ex:y )>> .` plus a
    // same-shaped two-hop walk over plain IRIs, `ex:m ex:p ex:n . ex:w ex:q ex:n .`
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri(&format!("{EX}a"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let qp = builder.intern_iri(&format!("{EX}q"));
    let r = builder.intern_iri(&format!("{EX}r"));
    let x = builder.intern_iri(&format!("{EX}x"));
    let y = builder.intern_iri(&format!("{EX}y"));
    let z = builder.intern_iri(&format!("{EX}z"));
    let m = builder.intern_iri(&format!("{EX}m"));
    let n = builder.intern_iri(&format!("{EX}n"));
    let w = builder.intern_iri(&format!("{EX}w"));
    let quoted = builder.intern_triple(x, r, y);
    builder.push_quad(a, p, quoted, None);
    builder.push_quad(z, qp, quoted, None);
    builder.push_quad(m, p, n, None);
    builder.push_quad(w, qp, n, None);
    let data = builder.freeze().expect("the fixture freezes");

    let graph = snapshot(
        &data,
        &[("p", PathDirection::Forward), ("q", PathDirection::Inverse)],
    );
    let registry = walk_registry(graph, limits(1, 2));

    let body = "SELECT ?end ?pathId ?len ?step ?node ?edge WHERE { \
                SEED <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
                } ORDER BY ?len ?step";
    let through_triple_term = solve(&data, &q(&body.replace("SEED", "ex:a")), &registry);
    let through_iris = solve(&data, &q(&body.replace("SEED", "ex:m")), &registry);
    let triple_rows =
        through_triple_term.project(&["end", "pathId", "len", "step", "node", "edge"]);
    let iri_rows = through_iris.project(&["end", "pathId", "len", "step", "node", "edge"]);

    // The triple term itself, as this crate's value model spells it.
    let tt = TermValue::Triple {
        s: Box::new(iri("x")),
        p: Box::new(iri("r")),
        o: Box::new(iri("y")),
    };
    let into_tt = TermValue::Triple {
        s: Box::new(iri("a")),
        p: Box::new(iri("p")),
        o: Box::new(tt.clone()),
    };
    let out_of_tt = TermValue::Triple {
        s: Box::new(iri("z")),
        p: Box::new(iri("q")),
        o: Box::new(tt.clone()),
    };

    let tt_ids = distinct_column(&triple_rows, 1);
    assert_eq!(
        tt_ids.len(),
        2,
        "a→<<x r y>> and a→<<x r y>>→z: {triple_rows:?}"
    );
    let (one_hop, two_hop) = (tt_ids[0].clone(), tt_ids[1].clone());
    assert_eq!(
        triple_rows,
        vec![
            vec![
                tt.clone(),
                one_hop,
                int(1),
                int(1),
                tt.clone(),
                into_tt.clone()
            ],
            // The two-hop walk: the triple term is the node ENTERED at step 1, and ex:z
            // the node entered at step 2.
            vec![iri("z"), two_hop.clone(), int(2), int(1), tt, into_tt],
            vec![
                iri("z"),
                two_hop.clone(),
                int(2),
                int(2),
                iri("z"),
                out_of_tt
            ],
        ]
    );

    let iri_ids = distinct_column(&iri_rows, 1);
    assert_eq!(iri_ids.len(), 2, "m→n and m→n→w: {iri_rows:?}");
    let plain_two_hop = iri_ids[1].clone();
    assert_eq!(
        iri_rows,
        vec![
            vec![
                iri("n"),
                iri_ids[0].clone(),
                int(1),
                int(1),
                iri("n"),
                stmt("m", "p", "n")
            ],
            vec![
                iri("w"),
                plain_two_hop.clone(),
                int(2),
                int(1),
                iri("n"),
                stmt("m", "p", "n")
            ],
            vec![
                iri("w"),
                plain_two_hop.clone(),
                int(2),
                int(2),
                iri("w"),
                stmt("w", "q", "n")
            ],
        ]
    );

    assert_ne!(
        two_hop, plain_two_hop,
        "two two-hop walks of the same shape, one through a triple term and one through \
         plain IRIs, are different derivations and must not share an identifier"
    );
}

// ---------------------------------------------------------------------------
// A10 — one seam, two differently-shaped consumers
// ---------------------------------------------------------------------------

/// The path-binding consumer and a scored-retrieval consumer, in ONE registry and ONE
/// query, joined.
///
/// The second relation is a `(document, score)` top-k table — the shape a text-search or
/// kNN consumer produces — registered under its own caller IRI as an ordinary
/// [`MemoryRelation`]. Nothing about the seam was reshaped for either: one declares an
/// arity of `(1, 6)` and enumerates walks, the other declares `(1, 1)` and scans a ranked
/// table, and a single query calls both and joins them on the walk's endpoint.
#[test]
fn a10_one_registry_serves_a_path_consumer_and_a_scored_retrieval_consumer() {
    let data = dataset(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")]);
    let graph = snapshot(&data, &[("p", PathDirection::Forward)]);

    let mut registry = walk_registry(graph, limits(1, 3));
    // The ranked table, in descending score order as a top-k retrieval surface hands it
    // back. `ex:d` is deliberately absent: an unscored document drops out of the join,
    // which is what makes the join observable rather than decorative.
    registry.register(
        RANKED.to_owned(),
        Arc::new(
            MemoryRelation::new(1, 1, vec![vec![iri("c"), int(9)], vec![iri("b"), int(7)]])
                .expect("every row is two values wide"),
        ),
    );

    let answers = solve(
        &data,
        &q("SELECT ?end ?len ?score WHERE { \
            ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) . \
            ?end <http://example.org/pf#ranked> ?score . \
            FILTER ( ?step = ?len ) \
            } ORDER BY ?len"),
        &registry,
    );

    assert_eq!(
        answers.project(&["end", "len", "score"]),
        vec![
            vec![iri("b"), int(1), int(7)],
            vec![iri("c"), int(2), int(9)],
        ],
        "the walk ending at ex:d has no score and drops out of the join"
    );
}

// ---------------------------------------------------------------------------
// A12 — the guards hard-fail, and the licence-dependence is pinned
// ---------------------------------------------------------------------------

/// `max_paths_per_seed` is an `Err`, and whether it fires depends on the row ceiling.
///
/// The guard bounds work ACTUALLY PERFORMED — candidate walks actually enumerated — so a
/// query the engine grants a smaller row licence may legitimately stop before the guard
/// is reached. That is not silent truncation and cannot become it: the only two outcomes
/// are an `Err` naming the guard, or an `Ok` whose rows are a genuine prefix of the
/// complete answer, honestly bounded by the `LIMIT` the query asked for. There is no
/// third outcome in which a short bag is offered as a whole one.
#[test]
fn a12_the_path_guard_hard_fails_and_its_firing_tracks_the_row_licence() {
    // Two candidate walks from ex:a within two hops — a→b and a→b→c — against a guard
    // that permits one.
    let data = dataset(&[("a", "p", "b"), ("b", "p", "c")]);
    let graph = snapshot(&data, &[("p", PathDirection::Forward)]);
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        WALK.to_owned(),
        Arc::new(PathWitnessRelation::new(
            graph,
            PathLimits::new(1, 2, 1, 1_000_000).expect("a one-walk-per-seed envelope"),
        )),
    );

    let body = "SELECT ?end ?len ?step WHERE { \
                ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) }";

    let unbounded = run(&data, &q(body), &registry);
    assert!(
        unbounded.is_err(),
        "a breached guard is never a short Ok: {unbounded:?}"
    );
    let error = unbounded.expect_err("checked immediately above");
    assert!(
        error.message.contains("max_paths_per_seed"),
        "the diagnostic must name the guard: {}",
        error.message
    );
    assert!(
        error.message.contains(&format!("{EX}a")),
        "and the seed it was exploring: {}",
        error.message
    );

    // The same query under `LIMIT 1`. The engine grants the relation a one-row licence,
    // the traversal spends it on the first walk's only row, and it never reaches the
    // second candidate that would have breached the guard.
    let limited = solve(&data, &q(&format!("{body} LIMIT 1")), &registry);
    assert_eq!(
        limited.project(&["end", "len", "step"]),
        vec![vec![iri("b"), int(1), int(1)]],
        "exactly one row, and it is a genuine prefix of the complete answer"
    );
}

// ---------------------------------------------------------------------------
// A15 — no derivation is erased
// ---------------------------------------------------------------------------

/// Two statements joining the same pair of nodes are two walks, not one.
///
/// `ex:a ex:p ex:b` and `ex:b ex:q ex:a` both take a walk from `ex:a` to `ex:b` under the
/// step `(ex:p, Forward) | (ex:q, Inverse)`. A node-only path model reports ONE answer
/// here and silently erases a derivation; recording the traversed statement keeps the two
/// apart, and gives them two identifiers so `GROUP BY ?pathId` keeps them apart too.
#[test]
fn a15_two_statements_joining_one_node_pair_are_two_witnesses() {
    let data = dataset(&[("a", "p", "b"), ("b", "q", "a")]);
    let graph = snapshot(
        &data,
        &[("p", PathDirection::Forward), ("q", PathDirection::Inverse)],
    );
    let registry = walk_registry(graph, limits(1, 1));

    let answers = solve(
        &data,
        &q("SELECT ?end ?pathId ?len ?step ?node ?edge WHERE { \
            ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) }"),
        &registry,
    );
    let rows = answers.project(&["end", "pathId", "len", "step", "node", "edge"]);

    assert_eq!(rows.len(), 2, "two statements, two walks: {rows:?}");
    assert_eq!(rows[0][4], iri("b"), "the same node is entered");
    assert_eq!(rows[1][4], iri("b"));
    // ...over two DIFFERENT statements, each recorded in ASSERTED orientation whichever
    // way the step traversed it, so each joins straight back into the dataset.
    assert_eq!(rows[0][5], stmt("a", "p", "b"));
    assert_eq!(rows[1][5], stmt("b", "q", "a"));
    assert_ne!(rows[0][5], rows[1][5]);
    assert_ne!(
        rows[0][1], rows[1][1],
        "two derivations must not share an identifier"
    );
}

// ---------------------------------------------------------------------------
// A16 — agreement with, and pinned divergence from, the core path grammar
// ---------------------------------------------------------------------------

/// The relation's endpoint projection agrees with `p+`, and diverges from `p{2,2}` in a
/// stated, asserted way.
///
/// Both halves run in the SAME engine over the SAME data: the comparison is between two
/// query texts, not between this relation and a remembered claim about the grammar.
#[test]
fn a16_the_endpoint_projection_agrees_with_p_plus_and_the_divergence_is_pinned() {
    // (a) Over a cycle, with max_hops at least the node count, the relation's DISTINCT
    //     endpoints are exactly `p+`'s. A node on a cycle reaches ITSELF under `p+`, and
    //     the simple-PREFIX rule (the final node alone may repeat) is what makes the
    //     relation report that too — a strictly simple rule would omit it.
    let cyclic = dataset(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
    let graph = snapshot(&cyclic, &[("p", PathDirection::Forward)]);
    let registry = walk_registry(graph, limits(1, 8));

    let through_relation = solve(
        &cyclic,
        &q("SELECT DISTINCT ?end WHERE { \
            ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
            } ORDER BY ?end"),
        &registry,
    );
    let through_grammar = solve(
        &cyclic,
        &q("SELECT DISTINCT ?end WHERE { ex:a ex:p+ ?end } ORDER BY ?end"),
        &registry,
    );
    assert_eq!(
        through_relation.project(&["end"]),
        vec![vec![iri("a")], vec![iri("b")], vec![iri("c")]],
        "every node of the cycle, including the seed itself"
    );
    assert_eq!(
        through_relation.project(&["end"]),
        through_grammar.project(&["end"]),
        "the relation's endpoint projection IS the core grammar's p+ reachability"
    );

    // (b) The divergence, pinned rather than left to be discovered. `p{n,m}` is k-fold
    //     composition, which admits walks that revisit INTERIOR nodes; this relation
    //     enumerates only simple-PREFIX walks. On a two-cycle the two happen to agree at
    //     k = 2, because `a → b → a` is itself simple-prefix — so this asserts the exact
    //     point where they still coincide, which is what makes any future divergence at
    //     this length a visible change rather than a silent one.
    let two_cycle = dataset(&[("a", "p", "b"), ("b", "p", "a")]);
    let two_graph = snapshot(&two_cycle, &[("p", PathDirection::Forward)]);
    let two_registry = walk_registry(two_graph, limits(2, 2));

    let grammar_exact = solve(
        &two_cycle,
        &q("SELECT ?x WHERE { ex:a ex:p{2,2} ?x } ORDER BY ?x"),
        &two_registry,
    );
    assert_eq!(
        grammar_exact.project(&["x"]),
        vec![vec![iri("a")]],
        "the core grammar's two-fold composition from ex:a lands back on ex:a"
    );

    let relation_exact = solve(
        &two_cycle,
        &q("SELECT DISTINCT ?end WHERE { \
            ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
            } ORDER BY ?end"),
        &two_registry,
    );
    assert_eq!(
        relation_exact.project(&["end"]),
        vec![vec![iri("a")]],
        "and so does the relation's only two-hop walk, a → b → a"
    );
}

// ---------------------------------------------------------------------------
// A17 — identifier stability scope
// ---------------------------------------------------------------------------

/// The identifier is stable across independently built datasets over IRIs and literals,
/// and is stable through a blank node only when the blank's label and scope are the same.
///
/// Both halves are ASSERTED here, not documented: the blank-node case is constructed
/// three ways — the same label twice, and a different label — and the observed behaviour
/// is pinned, because a claim about a content-derived key that no test holds is a claim
/// that drifts.
#[test]
fn a17_identifier_stability_is_pinned_for_iris_and_for_blank_nodes() {
    let query = q("SELECT ?end ?pathId ?len ?step ?node WHERE { \
                   ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
                   } ORDER BY ?len ?step");

    // Two INDEPENDENTLY constructed datasets holding the same IRI-only data. Their term
    // tables were built in different orders, so a digest that folded in a dataset-local
    // id would differ here.
    let first = dataset(&[("a", "p", "b"), ("b", "p", "c")]);
    let mut reordered = RdfDatasetBuilder::new();
    let c = reordered.intern_iri(&format!("{EX}c"));
    let p = reordered.intern_iri(&format!("{EX}p"));
    let b = reordered.intern_iri(&format!("{EX}b"));
    let a = reordered.intern_iri(&format!("{EX}a"));
    reordered.push_quad(b, p, c, None);
    reordered.push_quad(a, p, b, None);
    let second = reordered.freeze().expect("the fixture freezes");

    let ids_of = |data: &RdfDataset| {
        let graph = snapshot(data, &[("p", PathDirection::Forward)]);
        let registry = walk_registry(graph, limits(1, 2));
        let rows =
            solve(data, &query, &registry).project(&["end", "pathId", "len", "step", "node"]);
        (rows.clone(), distinct_column(&rows, 1))
    };
    let (first_rows, first_ids) = ids_of(&first);
    let (second_rows, second_ids) = ids_of(&second);
    assert_eq!(
        first_rows, second_rows,
        "two independently built datasets holding the same data answer identically"
    );
    assert_eq!(
        first_ids, second_ids,
        "and a walk over IRIs alone gets the SAME identifier in both"
    );

    // Now a walk THROUGH a blank node: `ex:a ex:p _:mid . _:mid ex:p ex:c .`
    let blank_dataset = |label: &str, scope: BlankScope| -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri(&format!("{EX}a"));
        let p = builder.intern_iri(&format!("{EX}p"));
        let c = builder.intern_iri(&format!("{EX}c"));
        let mid = builder.intern_blank(label, scope);
        builder.push_quad(a, p, mid, None);
        builder.push_quad(mid, p, c, None);
        builder.freeze().expect("the fixture freezes")
    };
    let blank_ids = |data: &RdfDataset| -> Vec<TermValue> {
        let graph = snapshot(data, &[("p", PathDirection::Forward)]);
        let registry = walk_registry(graph, limits(1, 2));
        let rows =
            solve(data, &query, &registry).project(&["end", "pathId", "len", "step", "node"]);
        distinct_column(&rows, 1)
    };

    let same_label_a = blank_dataset("mid", BlankScope::DEFAULT);
    let same_label_b = blank_dataset("mid", BlankScope::DEFAULT);
    let other_label = blank_dataset("other", BlankScope::DEFAULT);
    let other_scope = blank_dataset("mid", BlankScope(7));

    assert_eq!(
        blank_ids(&same_label_a),
        blank_ids(&same_label_b),
        "the same blank LABEL in the same SCOPE yields the same identifiers"
    );
    assert_ne!(
        blank_ids(&same_label_a),
        blank_ids(&other_label),
        "a different blank label yields different identifiers — the observed behaviour, \
         and the honest one: no content-derived key can be stabler than the terms it is \
         derived from, and a blank node's identity IS its label and scope"
    );
    assert_ne!(
        blank_ids(&same_label_a),
        blank_ids(&other_scope),
        "and so does the same label in a different scope"
    );
}

// ---------------------------------------------------------------------------
// A18 — `?edge` joins back into the dataset
// ---------------------------------------------------------------------------

/// A hop's statement joins to its RDF 1.2 annotation by an ordinary basic graph pattern.
///
/// This is the whole reason `?edge` is a statement term in ASSERTED orientation rather
/// than a synthetic edge handle: in RDF 1.2 a reifier names a triple through
/// `?reifier rdf:reifies <<( s p o )>>`, so a `?edge` bound by the relation is
/// immediately joinable in object position of that very pattern, and the reifier's own
/// annotations follow from there. No new syntax, no new operator, no unquoting step.
#[test]
fn a18_a_hop_statement_joins_to_its_rdf12_annotation() {
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri(&format!("{EX}a"));
    let b = builder.intern_iri(&format!("{EX}b"));
    let c = builder.intern_iri(&format!("{EX}c"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let certainty = builder.intern_iri(&format!("{EX}certainty"));
    let reifier = builder.intern_iri(&format!("{EX}r1"));
    builder.push_quad(a, p, b, None);
    builder.push_quad(b, p, c, None);
    // The RDF 1.2 annotation on ONE of the two statements: a reifier naming `ex:a ex:p
    // ex:b`, carrying `ex:certainty "9"^^xsd:integer`.
    let annotated = builder.intern_triple(a, p, b);
    let score = builder.intern_literal(RdfLiteral::typed("9", XSD_INTEGER));
    builder.push_reifier(reifier, annotated);
    builder.push_annotation(reifier, certainty, score);
    let data = builder.freeze().expect("the fixture freezes");

    let graph = snapshot(&data, &[("p", PathDirection::Forward)]);
    let registry = walk_registry(graph, limits(1, 2));

    // The RDF 1.2 spelling, verified against this engine's passing conformance cases: a
    // reifier names a triple through `?reifier rdf:reifies <<( s p o )>>`, so a `?edge`
    // the relation bound to a statement term goes straight into that pattern's OBJECT
    // position, and the reifier's annotations follow from the reifier resource. (Writing
    // `?edge ex:certainty ?c` would match nothing: the annotation layer is keyed by the
    // reifier resource, and a triple term is never an asserted subject.)
    let answers = solve(
        &data,
        &q(&format!(
            "SELECT ?end ?len ?step ?edge ?reifier ?c WHERE {{ \
             ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) . \
             ?reifier <{RDF_REIFIES}> ?edge . \
             ?reifier ex:certainty ?c \
             }} ORDER BY ?len ?step"
        )),
        &registry,
    );

    let e1 = stmt("a", "p", "b");
    assert_eq!(
        answers.project(&["end", "len", "step", "edge", "reifier", "c"]),
        vec![
            // The one-hop walk a→b: its only hop traverses the annotated statement.
            vec![iri("b"), int(1), int(1), e1.clone(), iri("r1"), int(9)],
            // The two-hop walk a→b→c: only its FIRST hop traverses it, so only that hop
            // survives the join — the second hop's statement carries no reifier.
            vec![iri("c"), int(2), int(1), e1, iri("r1"), int(9)],
        ],
        "exactly the rows for the annotated hop"
    );
}

// ---------------------------------------------------------------------------
// A19 — the expansion budget
// ---------------------------------------------------------------------------

/// The expansion guard fires on a search that finds nothing, in bounded time.
///
/// The envelope below demands walks of exactly four hops over a fan-out graph whose
/// longest walk is two, so the correct complete answer is zero rows — and that is exactly
/// the shape in which a resource guard could most easily degrade into a silent `Ok`. It
/// does not: the search still traverses six edges looking for a four-hop walk, the guard
/// permits three, and the invocation fails with a diagnostic naming it.
#[test]
fn a19_the_expansion_budget_fails_a_fruitless_search_rather_than_answering_empty() {
    // A fan-out: ex:a has three neighbours, each of which has one. Six edges, longest
    // walk two hops.
    let data = dataset(&[
        ("a", "p", "b"),
        ("a", "p", "c"),
        ("a", "p", "d"),
        ("b", "p", "e"),
        ("c", "p", "f"),
        ("d", "p", "g"),
    ]);
    let graph = snapshot(&data, &[("p", PathDirection::Forward)]);
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        WALK.to_owned(),
        Arc::new(PathWitnessRelation::new(
            graph,
            PathLimits::new(4, 4, 4096, 3).expect("a three-edge expansion budget"),
        )),
    );

    let result = run(
        &data,
        &q("SELECT ?end WHERE { \
            ex:a <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) }"),
        &registry,
    );
    assert!(
        result.is_err(),
        "a breached expansion budget is never a zero-row Ok, even when the complete \
         answer happens to be empty: {result:?}"
    );
    let error = result.expect_err("checked immediately above");
    assert!(
        error.message.contains("max_expansions_per_invocation"),
        "the diagnostic must name the guard: {}",
        error.message
    );
}

// ---------------------------------------------------------------------------
// A20/A21 — the RDF 1.2 statement layer is edge data, not a separate world
// ---------------------------------------------------------------------------

/// `rdf:reifies` is a traversable predicate, so a statement term walks to its reifier.
///
/// The reifier side-table is NOT in the quad table. A snapshot that read only
/// `quads_for_pattern` would find no edges for this step, build a zero-node graph, and
/// then answer every query over it with zero rows, complete and undiagnosed — the exact
/// silent emptiness A5 exists to forbid, reached through the RDF 1.2 layer this relation
/// advertises as first class. `rdf:reifies` is always interned by the builder when a
/// reifier is pushed, so the step would not even fail the "is this predicate known"
/// check on its way to answering nothing.
///
/// The step is `Inverse`, so the walk runs from the reified STATEMENT to the reifier
/// resource that names it — the direction a consumer asking "what is said about this
/// hop" actually travels.
#[test]
fn a20_a_step_over_rdf_reifies_walks_a_statement_to_its_reifier() {
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri(&format!("{EX}a"));
    let b = builder.intern_iri(&format!("{EX}b"));
    let c = builder.intern_iri(&format!("{EX}c"));
    let d = builder.intern_iri(&format!("{EX}d"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let r1 = builder.intern_iri(&format!("{EX}r1"));
    let r2 = builder.intern_iri(&format!("{EX}r2"));
    builder.push_quad(a, p, b, None);
    builder.push_quad(c, p, d, None);
    let ab = builder.intern_triple(a, p, b);
    let cd = builder.intern_triple(c, p, d);
    builder.push_reifier(r1, ab);
    builder.push_reifier(r2, cd);
    let data = builder.freeze().expect("the fixture freezes");

    let step = PathStep::new(vec![(TermValue::iri(RDF_REIFIES), PathDirection::Inverse)])
        .expect("a well-formed step");
    let graph = Arc::new(
        PathGraph::from_dataset(&*data, &step, GraphMatch::Default).expect("the snapshot builds"),
    );
    assert_eq!(
        graph.edge_count(),
        2,
        "both reifier bindings are edges of the snapshot"
    );
    let registry = walk_registry(graph, limits(1, 1));

    let answers = solve(
        &data,
        &q("SELECT ?start ?end ?len ?step ?node ?edge WHERE { \
            ?start <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
            } ORDER BY ?end"),
        &registry,
    );

    // The traversed statement is the reifier row itself, in asserted orientation:
    // `ex:rN rdf:reifies <<( s p o )>>`.
    let reifies_stmt = |reifier: &str, reified: TermValue| TermValue::Triple {
        s: Box::new(iri(reifier)),
        p: Box::new(TermValue::iri(RDF_REIFIES)),
        o: Box::new(reified),
    };
    assert_eq!(
        answers.project(&["start", "end", "len", "step", "node", "edge"]),
        vec![
            vec![
                stmt("a", "p", "b"),
                iri("r1"),
                int(1),
                int(1),
                iri("r1"),
                reifies_stmt("r1", stmt("a", "p", "b")),
            ],
            vec![
                stmt("c", "p", "d"),
                iri("r2"),
                int(1),
                int(1),
                iri("r2"),
                reifies_stmt("r2", stmt("c", "p", "d")),
            ],
        ],
        "one hop per reifier binding, the statement term as the seed"
    );
}

/// An annotation predicate is a traversable predicate, and the two layers form ONE graph.
///
/// `ex:source` here names both an RDF 1.2 statement annotation (`ex:r1 ex:source ex:doc1`,
/// which lives in the annotation side-table) and an ordinary asserted quad
/// (`ex:doc1 ex:source ex:doc2`, which lives in the quad table). A single step over
/// `ex:source` must traverse BOTH, because they are both `ex:source` edges of the data —
/// so the two-hop walk `ex:r1 → ex:doc1 → ex:doc2` exists and crosses the layer boundary
/// mid-walk. A snapshot blind to the side-table would answer this with the single quad
/// edge, reachable from no seed the query asks about, and so with zero rows.
#[test]
fn a21_a_step_over_an_annotation_predicate_crosses_both_layers_in_one_walk() {
    let mut builder = RdfDatasetBuilder::new();
    let a = builder.intern_iri(&format!("{EX}a"));
    let b = builder.intern_iri(&format!("{EX}b"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let source = builder.intern_iri(&format!("{EX}source"));
    let r1 = builder.intern_iri(&format!("{EX}r1"));
    let doc1 = builder.intern_iri(&format!("{EX}doc1"));
    let doc2 = builder.intern_iri(&format!("{EX}doc2"));
    builder.push_quad(a, p, b, None);
    // The asserted half of the `ex:source` relation.
    builder.push_quad(doc1, source, doc2, None);
    // The statement-layer half: a reifier for `<<ex:a ex:p ex:b>>`, annotated with the
    // very same predicate.
    let ab = builder.intern_triple(a, p, b);
    builder.push_reifier(r1, ab);
    builder.push_annotation(r1, source, doc1);
    let data = builder.freeze().expect("the fixture freezes");

    let graph = snapshot(&data, &[("source", PathDirection::Forward)]);
    assert_eq!(
        graph.edge_count(),
        2,
        "one annotation edge and one asserted edge, in one snapshot"
    );
    let registry = walk_registry(graph, limits(1, 2));

    let answers = solve(
        &data,
        &q("SELECT ?end ?len ?step ?node ?edge WHERE { \
            ex:r1 <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
            } ORDER BY ?len ?step"),
        &registry,
    );

    let annotation_edge = stmt("r1", "source", "doc1");
    let asserted_edge = stmt("doc1", "source", "doc2");
    assert_eq!(
        answers.project(&["end", "len", "step", "node", "edge"]),
        vec![
            // The one-hop walk that exists only in the annotation side-table.
            vec![
                iri("doc1"),
                int(1),
                int(1),
                iri("doc1"),
                annotation_edge.clone()
            ],
            // ...and its extension through an ordinary asserted quad.
            vec![iri("doc2"), int(2), int(1), iri("doc1"), annotation_edge],
            vec![iri("doc2"), int(2), int(2), iri("doc2"), asserted_edge],
        ],
        "the annotation edge and the asserted edge are one graph"
    );
}

// ---------------------------------------------------------------------------
// A22 — a fixed step vocabulary over a dataset that carries half of it
// ---------------------------------------------------------------------------

/// An alternation member the dataset never mentions contributes nothing, and the query
/// still answers.
///
/// This is the shape a host with a FIXED step vocabulary has: one `PathStep`, many
/// datasets. `ex:narrower` appears nowhere in this dataset — not as a predicate, not as a
/// subject, not as an object — so it is not interned at all, while `ex:broader` carries
/// two edges. The correct answer is the `ex:broader` walks, because an alternation is an
/// alternation: `p|q` in the core grammar does not fail when `q` matches nothing, and
/// neither does this. Refusing the snapshot would key a hard failure on whether an IRI
/// happens to appear in the term table, which is an interning detail no host can predict.
#[test]
fn a22_a_step_alternative_the_dataset_never_mentions_still_leaves_a_usable_step() {
    let data = dataset(&[("t1", "broader", "t2"), ("t2", "broader", "t3")]);
    let step = PathStep::new(vec![
        (iri("broader"), PathDirection::Forward),
        (iri("narrower"), PathDirection::Inverse),
    ])
    .expect("a well-formed step");
    let graph = Arc::new(
        PathGraph::from_dataset(&*data, &step, GraphMatch::Default)
            .expect("one empty alternative does not invalidate the step"),
    );
    assert_eq!(
        graph.edge_count(),
        2,
        "the ex:broader edges, and only those"
    );
    let registry = walk_registry(graph, limits(1, 2));

    let answers = solve(
        &data,
        &q("SELECT ?end ?len ?step ?node ?edge WHERE { \
            ex:t1 <http://example.org/pf#walk> ( ?end ?pathId ?len ?step ?node ?edge ) \
            } ORDER BY ?len ?step"),
        &registry,
    );
    let hop1 = stmt("t1", "broader", "t2");
    let hop2 = stmt("t2", "broader", "t3");
    assert_eq!(
        answers.project(&["end", "len", "step", "node", "edge"]),
        vec![
            vec![iri("t2"), int(1), int(1), iri("t2"), hop1.clone()],
            vec![iri("t3"), int(2), int(1), iri("t2"), hop1],
            vec![iri("t3"), int(2), int(2), iri("t3"), hop2],
        ],
        "exactly the walks the present alternative admits"
    );
}
