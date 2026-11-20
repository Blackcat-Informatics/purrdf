#![allow(missing_docs, clippy::all, clippy::pedantic, clippy::nursery)]
// Throwaway per-call driver (measurement only; lives under .stage, installed as an untracked
// example of purrdf-shapes by m2_mk.sh). Placeholders:  (unwrap on the fallible builder
// API) and  (options borrow).
//
// usage: m2_drv <subjects> <row> <calls>      env M2_WARM=<query row> runs that other text once first
//
// Fixture: `subjects` typed subjects, one rdf:type each, 61 types in the reported skew
// (405:385:135:134:119 per 1810, 12 headers, the rest over 55 small types), default graph only,
// subjects * 10.3 / 1.81 quads, no reifiers or annotations. Seedless: every value is a fixed
// arithmetic function of the subject index.
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult, TermId};
use purrdf_shapes::data_view::ShaclDatasetView;
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_sparql_eval::{
    CancellationFlag, GovernedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
unsafe impl std::alloc::GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(l.size() as u64, Relaxed);
        unsafe { std::alloc::System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        BYTES.fetch_add(n as u64, Relaxed);
        unsafe { std::alloc::System.realloc(p, l, n) }
    }
}
#[global_allocator]
static GLOBAL: Counting = Counting;

const NS: &str = "https://example.org/ns/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const SH: &str = "http://www.w3.org/ns/shacl#";
const TYPES: usize = 61;
const HEADERS: usize = 12;
const EXT: usize = 70;

fn type_name(t: usize) -> String {
    match t {
        0 => "T".into(),
        3 => "K1".into(),
        5 => "K2".into(),
        6 => "C1".into(),
        7 => "C2".into(),
        8 => "C3".into(),
        60 => "Header".into(),
        _ => format!("S{t}"),
    }
}

/// Subjects per type: the five largest in the reported proportions, 12 headers, the rest
/// spread evenly over the 55 small types.
fn type_sizes(subjects: usize) -> Vec<usize> {
    let mut n = vec![0usize; TYPES];
    for (t, share) in [405usize, 385, 135, 134, 119].iter().enumerate() {
        n[t] = subjects * share / 1810;
    }
    n[60] = HEADERS;
    let rest = subjects - n.iter().sum::<usize>();
    for t in 5..60 {
        n[t] = rest / 55 + usize::from(t - 5 < rest % 55);
    }
    assert_eq!(n.iter().sum::<usize>(), subjects);
    n
}

fn eighths(v: i64) -> String {
    let a = v.unsigned_abs();
    format!(
        "{}{}.{:03}",
        if v < 0 { "-" } else { "" },
        a / 8,
        (a % 8) * 125
    )
}

fn build(subjects: usize) -> (Arc<RdfDataset>, usize) {
    let sizes = type_sizes(subjects);
    let mut start = vec![0usize; TYPES + 1];
    for t in 0..TYPES {
        start[t + 1] = start[t] + sizes[t];
    }
    let target = subjects * 1030 / 181;
    let mut b = RdfDatasetBuilder::new();
    let rdf_type = b.intern_iri(&format!("{RDF}type"));
    let mut emitted = 0usize;
    macro_rules! iri {
        ($($a:tt)*) => { b.intern_iri(&format!("{NS}{}", format!($($a)*))) };
    }
    macro_rules! float {
        ($v:expr) => {
            b.intern_literal(RdfLiteral::typed(eighths($v), format!("{XSD}float")))
        };
    }
    macro_rules! quad {
        ($s:expr, $p:expr, $o:expr) => {{
            b.push_quad($s, $p, $o, None);
            emitted += 1;
        }};
    }
    let s: Vec<TermId> = (0..subjects).map(|i| iri!("s{i}")).collect();
    let classes: Vec<TermId> = (0..TYPES).map(|t| iri!("{}", type_name(t))).collect();
    let [
        p,
        q,
        pb,
        pt,
        status,
        pa,
        pn,
        pc,
        path,
        pd,
        pv,
        set,
        ptype,
        kind,
        pe,
        profile,
        name,
        desc,
        pref,
        ext,
    ] = [
        "p", "q", "b", "t", "status", "a", "n", "c", "path", "d", "v", "set", "type", "kind", "e",
        "profile", "name", "desc", "ref", "ext",
    ]
    .map(|l| iri!("{l}"));
    let lv = [iri!("v1"), iri!("v2"), iri!("v3")];
    let statuses: Vec<TermId> = (0..5).map(|k| iri!("st{k}")).collect();
    let kinds = [iri!("K"), iri!("Kother")];
    let bools = ["false", "true"]
        .map(|l| b.intern_literal(RdfLiteral::typed(l.to_owned(), format!("{XSD}boolean"))));
    let core = b.intern_literal(RdfLiteral::simple("core".to_owned()));
    let descs: Vec<TermId> = (0..64)
        .map(|k| b.intern_literal(RdfLiteral::simple(format!("description {k}"))))
        .collect();

    let of = |t: usize, k: usize| start[t] + k % sizes[t];
    // Nodes: types 10..30; the E pool: types 3 and 5; I: types 30..40.
    let nodes = start[30] - start[10];
    let node = |k: usize| start[10] + k % nodes;
    let epool = sizes[3] + sizes[5];
    let e_at = |k: usize| {
        let k = k % epool;
        if k < sizes[3] {
            start[3] + k
        } else {
            start[5] + (k - sizes[3])
        }
    };

    for t in 0..TYPES {
        for k in 0..sizes[t] {
            let i = start[t] + k;
            let me = s[i];
            quad!(me, rdf_type, classes[t]);
            let nm = b.intern_literal(RdfLiteral::simple(format!("name {i}")));
            quad!(me, name, nm);
            if i % 2 == 0 {
                quad!(me, pref, s[(i * 31 + 7) % subjects]);
            }
            if i % 97 != 0 {
                quad!(me, desc, descs[i % 64]);
            }
            match t {
                0 => {
                    let pvv = float!(((k * 37) % 1601) as i64 - 800);
                    let qv = float!(((k * 53) % 1601) as i64 - 800);
                    quad!(me, p, pvv);
                    quad!(me, q, qv);
                    quad!(me, pb, s[of(1, k)]);
                    quad!(me, pt, s[of(1, k * 5 + 3)]);
                    quad!(me, status, statuses[k % 5]);
                }
                1 => {
                    quad!(me, pa, s[of(2, k)]);
                    quad!(me, pn, s[node(k * 3 + 1)]);
                    if k % 4 == 0 {
                        quad!(me, pc, s[e_at(k * 11)]);
                    }
                }
                2 => quad!(me, path, s[of(1, k * 3 + 2)]),
                3 | 5 => quad!(me, pd, bools[usize::from(k % 3 != 0)]),
                4 => {
                    quad!(me, pn, s[node(k * 5)]);
                    let v = float!(8 + (k % 200) as i64);
                    quad!(me, pv, v);
                }
                6..=8 => {
                    quad!(me, set, s[of(2, k * 13)]);
                    quad!(me, ptype, s[of(9, k)]);
                    let v = float!(8 + (k % 400) as i64);
                    quad!(me, lv[t - 6], v);
                }
                9 => quad!(me, kind, kinds[k % 2]),
                30..=39 => quad!(me, pe, s[of(2, k * 7 + t)]),
                60 => quad!(me, profile, core),
                _ => {}
            }
        }
    }
    for k in 0..EXT {
        let o = b.intern_iri(&format!("https://example.org/ext/doc{k}.xml"));
        quad!(s[(k * 257) % subjects], ext, o);
    }
    assert!(
        emitted <= target,
        "role quads {emitted} exceed the target {target}"
    );
    let ints: Vec<TermId> = (0..1000)
        .map(|v| b.intern_literal(RdfLiteral::typed(v.to_string(), format!("{XSD}integer"))))
        .collect();
    let mut round = 0usize;
    'fill: loop {
        let px = iri!("x{round}");
        for i in 0..subjects {
            if emitted == target {
                break 'fill;
            }
            quad!(s[i], px, ints[(i * 7 + round * 13) % 1000]);
        }
        round += 1;
    }
    let dataset = b.freeze().unwrap();
    (dataset, target)
}

fn query_body(row: &str) -> &'static str {
    match row {
        // The five shapes of the maintainers' table.
        "one" => "SELECT ?f ?p WHERE { ?f ex:p ?p }",
        "filter" => "SELECT ?f ?p WHERE { ?f ex:p ?p . FILTER(?p > \"50.0\"^^xsd:float) }",
        "join" => "SELECT ?f ?p ?st WHERE { ?f ex:p ?p ; ex:status ?st }",
        "notexists" => "SELECT ?f ?p WHERE { ?f ex:p ?p . FILTER NOT EXISTS { ?f ex:status ex:st0 } }",
        "groupby" => "SELECT ?st (COUNT(?f) AS ?n) WHERE { ?f ex:status ?st } GROUP BY ?st",
        // The reported texts, verbatim.
        "q1" => "SELECT ?x (COUNT(?v) AS ?n) ?v\nWHERE { ?v rdf:type ?o . }\nGROUP BY ?x ?v\nHAVING (?n > 1)",
        "q3" => "SELECT ?x ?sumP ?sumQ WHERE {
  { SELECT ?x (SUM(?p) AS ?sumP) (SUM(?q) AS ?sumQ) WHERE {
      ?x ^ex:a/^ex:b ?f .
      ?f ex:p ?p ; ex:q ?q ; ex:b ?t .
      FILTER NOT EXISTS { ?t ex:c ?e . ?e ex:d false . ?e a ?k . FILTER (?k IN (ex:K1, ex:K2)) }
    } GROUP BY ?x }
  BIND (EXISTS { ?x ^ex:e ?i } AS ?has)
  FILTER (?has = false && ((ABS(?sumP) > 0.1 && ABS(?sumP) < 0.5) || (ABS(?sumQ) > 0.1 && ABS(?sumQ) < 0.5)))
}",
        "q4" => "SELECT ?x ?v1 ?v2 ?v3 ?p ?q WHERE {
  ?l ex:set ?x ; a ?cls ; ex:type ?lt .
  ?lt ex:kind ex:K .
  ?l (ex:v1|ex:v2|ex:v3) ?lv .
  ?x ex:path ?t .
  ?f ex:t ?t .  ?t ex:n ?n .
  ?s ex:n ?n ; ex:v ?v .
  ?f ex:p ?p .  ?f ex:q ?q .
  BIND (IF(?cls = ex:C1, ?lv, 1/0) AS ?v1)
  BIND (IF(?cls = ex:C2, ?lv, 1/0) AS ?v2)
  BIND (IF(?cls = ex:C3, ?lv, 1/0) AS ?v3)
  BIND ((?p*?p + ?q*?q) AS ?a2)
  FILTER ((?cls = ex:C1 && ?a2 != 0 && ?lv*?lv < (?a2/(3*(?v*?v)))*1000000)
       || (?cls = ex:C2 && ?a2 != 0 && ?lv*?lv < ?a2)
       || (?cls = ex:C3 && ?p != 0 && ?lv < ABS(?p)))
}",
        // The two synthetic shapes the reported table names without a text.
        "q2" => "SELECT ?f ?x ?p WHERE { ?f ex:b/ex:a ?x . ?f ex:p ?p . FILTER(?p > \"50.0\"^^xsd:float) }",
        "q5" => "SELECT ?f ?p WHERE { ?f ex:p ?p ; ex:b ?t . FILTER NOT EXISTS { ?f ex:q ?q . FILTER(?q > ?p) } }",
        _ => panic!("unknown query row {row}"),
    }
}

fn query_text(row: &str) -> String {
    format!(
        "PREFIX ex: <{NS}>\nPREFIX xsd: <{XSD}>\nPREFIX rdf: <{RDF}>\n{}",
        query_body(row)
    )
}

fn shapes_turtle(set: &str) -> String {
    let mut out = format!("@prefix sh: <{SH}> .\n@prefix ex: <{NS}> .\n@prefix xsd: <{XSD}> .\n\n");
    match set {
        // Three sh:targetClass shapes per type: class membership for all 61 types.
        "class" => {
            for t in 0..TYPES {
                let n = type_name(t);
                out.push_str(&format!(
                    "ex:Shape_{n}_0 a sh:NodeShape ; sh:targetClass ex:{n} ;\n  sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:maxCount 1 ; sh:datatype xsd:string ] .\n\
                     ex:Shape_{n}_1 a sh:NodeShape ; sh:targetClass ex:{n} ;\n  sh:property [ sh:path ex:ref ; sh:nodeKind sh:IRI ] .\n\
                     ex:Shape_{n}_2 a sh:NodeShape ; sh:targetClass ex:{n} ;\n  sh:property [ sh:path ex:desc ; sh:minCount 1 ] .\n\n"
                ));
            }
        }
        // A cross product: the 12 headers times every subject of the largest type.
        "cross" => out.push_str(&format!(
            "ex:Cross a sh:NodeShape ;\n  sh:target [ a sh:SPARQLTarget ; sh:select \"\"\"SELECT ?this WHERE {{ ?h <{NS}profile> \"core\" . ?this a <{NS}T> . }}\"\"\" ] ;\n  sh:property [ sh:path ex:desc ; sh:minCount 1 ] .\n"
        )),
        // A scan of every quad with isIRI, string tests and NOT EXISTS: about 70 focus nodes.
        "scan" => out.push_str(&format!(
            "ex:Scan a sh:NodeShape ;\n  sh:target [ a sh:SPARQLTarget ; sh:select \"\"\"SELECT ?this WHERE {{ ?s ?p ?this . FILTER(isIRI(?this)) FILTER(STRSTARTS(STR(?this), \"https://example.org/\") && CONTAINS(STR(?this), \"/ext/\") && STRENDS(STR(?this), \".xml\")) FILTER NOT EXISTS {{ ?this a ?t }} }}\"\"\" ] ;\n  sh:nodeKind sh:BlankNode .\n"
        )),
        _ => panic!("unknown shape set {set}"),
    }
    out
}

fn counters() -> (u64, u64) {
    (ALLOCS.load(Relaxed), BYTES.load(Relaxed))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let subjects: usize = a[1].parse().unwrap();
    let row = a[2].as_str();
    let calls: usize = a[3].parse().unwrap();
    let (dataset, quads) = build(subjects);
    let mut witness = 0usize;
    let mut window = ((0, 0), (0, 0));

    if let Some(set) = row
        .strip_prefix("bind_")
        .or_else(|| row.strip_prefix("val_"))
    {
        let prepared =
            PreparedShapes::new(Arc::new(parse_shapes(&shapes_turtle(set), None).unwrap()));
        if row.starts_with("bind_") {
            // N binds of the same prepared shapes on one dataset; the first is the cold one.
            for _ in 0..calls {
                let before = counters();
                let validator = prepared.bind_shared_dataset(Arc::clone(&dataset)).unwrap();
                window = (before, counters());
                drop(validator);
                witness += 1;
            }
        } else {
            // One bind outside the window, then N validations of that bound validator.
            let validator = prepared.bind_shared_dataset(Arc::clone(&dataset)).unwrap();
            for _ in 0..calls {
                let before = counters();
                let report = validator.validate().unwrap();
                window = (before, counters());
                witness = report.results.len() * 2 + usize::from(report.conforms);
            }
        }
    } else {
        let engine = NativeSparqlEngine::new();
        let view = ShaclDatasetView::native(Arc::clone(&dataset));
        let governors = QueryGovernors::UNBOUNDED
            .with_max_scratch_bytes(1 << 40)
            .with_max_intermediate_cells(1 << 40)
            .with_stop_signal(Arc::new(CancellationFlag::new()));
        if let Ok(warm) = std::env::var("M2_WARM") {
            let wprepared = engine.prepare_query(&query_text(&warm), None).unwrap();
            let w = engine
                .query_prepared_governed_view(
                    &view,
                    &wprepared,
                    &[],
                    QueryOptions::EMPTY,
                    &governors,
                )
                .unwrap();
            drop(w);
        }
        // Parsing is outside the per-call difference: it runs with zero calls too.
        let prepared = engine.prepare_query(&query_text(row), None).unwrap();
        for _ in 0..calls {
            let before = counters();
            let outcome = engine
                .query_prepared_governed_view(
                    &view,
                    &prepared,
                    &[],
                    QueryOptions::EMPTY,
                    &governors,
                )
                .unwrap();
            window = (before, counters());
            let GovernedOutcome::Complete { result, .. } = &outcome else {
                panic!("the governed query did not complete");
            };
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("the row is a SELECT");
            };
            witness = rows.len();
            // The result is dropped here, inside the per-call difference, as a caller pays it.
        }
    }
    println!(
        "row={row} subjects={subjects} quads={quads} calls={calls} witness={witness} last_allocs={} last_bytes={}",
        window.1.0 - window.0.0,
        window.1.1 - window.0.1
    );
}
