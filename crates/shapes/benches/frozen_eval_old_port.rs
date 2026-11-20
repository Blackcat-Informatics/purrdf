#![allow(dead_code, missing_docs)]
// Why not Rust: not applicable, this is Rust; temporary measurement port of the issue bench, kept out of the PR.
use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult, TermId};
use purrdf_iri::vocab::{rdf, sh};
use purrdf_shapes::engine::{PreparedShapes, parse_shapes};
use purrdf_sparql_eval::{
    EvalOptions, GovernedOutcome, NativeSparqlEngine, PreparedQuery, QueryGovernors, QueryOptions,
};
use purrdf_testkit::rng::SplitMix64;
use purrdf_xsd::datatype::{XSD_DECIMAL, XSD_DOUBLE, XSD_FLOAT, XSD_INTEGER, XSD_NS};
use std::sync::Arc;
use std::time::Instant;

/// The old builder returns ids and unit, not results.
trait Expect: Sized {
    fn expect(self, _: &str) -> Self {
        self
    }
}
impl Expect for TermId {}
impl Expect for () {}

/// The fixture namespace. Caller-supplied, and `example.org` by rule.
pub(crate) const NS: &str = "https://example.org/ns/";

/// The number of named graphs, as in the reported dataset.
pub(crate) const NAMED_GRAPHS: usize = 117;

/// The reported dataset: 10.3 million default-graph and 11.7 million named-graph quads.
pub(crate) const REPORTED_QUADS: usize = 22_000_000;

/// The two routine sizes: one hundredth and one thousandth of the reported load.
pub(crate) const ROUTINE_QUADS: [usize; 2] = [22_000, 220_000];

/// The largest size selected without the explicit opt-in.
pub(crate) const LARGE_REFUSAL_QUADS: usize = 4_400_000;

const SEED: u64 = 0x1281_0000_5EED_0001;

/// Distinct classes, drawn Zipf-like.
const CLASSES: usize = 12;

/// Statuses; index 1 is `ex:closed`.
const STATUSES: u64 = 5;

/// The sizes this run measures, refused before anything is built.
///
/// `env_name` carries one extra size. Above [`LARGE_REFUSAL_QUADS`] it also
/// needs `PURRDF_FROZEN_EVAL_ALLOW_LARGE=1`; the check runs here, before a
/// builder, a dataset or a thread exists.
pub(crate) fn sizes(env_name: &str) -> Vec<usize> {
    let mut sizes = ROUTINE_QUADS.to_vec();
    if let Some(raw) = std::env::var_os(env_name) {
        let quads: usize = raw
            .into_string()
            .expect("the size is UTF-8")
            .parse()
            .unwrap_or_else(|_| panic!("{env_name} is an integer"));
        assert!(
            quads >= 1_000,
            "{env_name}={quads} is below the 1000 quads the layout needs"
        );
        assert!(
            quads <= LARGE_REFUSAL_QUADS
                || std::env::var("PURRDF_FROZEN_EVAL_ALLOW_LARGE").as_deref() == Ok("1"),
            "{env_name}={quads} exceeds {LARGE_REFUSAL_QUADS}; set PURRDF_FROZEN_EVAL_ALLOW_LARGE=1"
        );
        if !sizes.contains(&quads) {
            sizes.push(quads);
        }
    }
    sizes
}

/// The proportions of one dataset size.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    /// Total quads, default graph plus named graphs.
    pub quads: usize,
    /// Quads in the default graph.
    pub default_quads: usize,
    /// Quads across the named graphs.
    pub named_quads: usize,
    /// Subjects that own default-graph rows.
    pub subjects: usize,
    /// The exclusive bound of the value draw `v`; a multiple of 16.
    pub span: u64,
}

impl Layout {
    pub(crate) fn new(quads: usize) -> Self {
        let default_quads = quads * 47 / 100;
        let mut subjects = 0;
        let mut rows = 0;
        while rows < default_quads {
            rows += default_rows(subjects);
            subjects += 1;
        }
        let span = (subjects as u64 / 4).max(1024).div_ceil(16) * 16;
        Self {
            quads,
            default_quads,
            named_quads: quads - default_quads,
            subjects,
            span,
        }
    }

    /// The numeric filter constant: the subjects with `v / 8 < threshold` are kept,
    /// which is `v < span / 2`, half of them.
    pub(crate) const fn threshold(&self) -> u64 {
        self.span / 16
    }

    /// A count the integer property seldom exceeds, for the SHACL constraints.
    const fn high_count(&self) -> u64 {
        self.span / 8 - self.span / 64
    }

    /// The reporter's load divided by this size, for the log.
    pub(crate) fn fraction_of_reported(&self) -> f64 {
        REPORTED_QUADS as f64 / self.quads as f64
    }
}

/// Rows one subject owns in the default graph: seven properties, a parent link
/// (not for the root) and one to seven links.
fn default_rows(subject: usize) -> usize {
    7 + usize::from(subject > 0) + 1 + subject % 7
}

struct Draws {
    value: u64,
    status: u64,
    class: usize,
}

fn draws(subject: usize, span: u64) -> Draws {
    let mut rng = SplitMix64::new(SEED ^ (subject as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let value = rng.below(span);
    let status = rng.below(STATUSES);
    // Zipf-like: the square of a unit draw piles subjects onto the low classes.
    let unit = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    Draws {
        value,
        status,
        class: ((unit * unit * CLASSES as f64) as usize).min(CLASSES - 1),
    }
}

/// Every interned term the rows are made of.
struct Terms {
    rdf_type: TermId,
    score: TermId,
    weight: TermId,
    count: TermId,
    price: TermId,
    label: TermId,
    status: TermId,
    parent: TermId,
    link: TermId,
    classes: Vec<TermId>,
    statuses: Vec<TermId>,
    graphs: Vec<TermId>,
    subjects: Vec<TermId>,
    floats: Vec<TermId>,
    doubles: Vec<TermId>,
    decimals: Vec<TermId>,
    integers: Vec<TermId>,
    labels: Vec<TermId>,
}

/// `v / 8` as an exact decimal lexical form.
fn eighths(value: u64) -> String {
    format!("{}.{:03}", value / 8, (value % 8) * 125)
}

fn intern_terms(b: &mut RdfDatasetBuilder, layout: &Layout) -> Terms {
    let rdf_type = b.intern_iri(rdf::TYPE).expect("rdf:type interns");
    let mut iri = |local: &str| {
        b.intern_iri(&format!("{NS}{local}"))
            .expect("the fixture IRI interns")
    };
    let score = iri("score");
    let weight = iri("weight");
    let count = iri("count");
    let price = iri("price");
    let label = iri("label");
    let status = iri("status");
    let parent = iri("parent");
    let link = iri("link");
    let classes = (0..CLASSES).map(|c| iri(&format!("C{c}"))).collect();
    let statuses = ["open", "closed", "held", "done", "void"]
        .iter()
        .map(|name| iri(name))
        .collect();
    let graphs = (0..NAMED_GRAPHS).map(|g| iri(&format!("g{g}"))).collect();
    let subjects = (0..layout.subjects)
        .map(|s| iri(&format!("s{s}")))
        .collect();
    let mut literal = |lexical: String, datatype: &str| {
        b.intern_literal(RdfLiteral::typed(lexical, datatype))
            .expect("the fixture literal interns")
    };
    let floats = (0..layout.span)
        .map(|v| literal(eighths(v), XSD_FLOAT))
        .collect();
    let doubles = (0..layout.span)
        .map(|v| literal(eighths(v), XSD_DOUBLE))
        .collect();
    let decimals = (0..layout.span)
        .map(|v| literal(eighths(v), XSD_DECIMAL))
        .collect();
    let integers = (0..layout.span / 8)
        .map(|v| literal(v.to_string(), XSD_INTEGER))
        .collect();
    let labels = (0..layout.span / 8)
        .map(|v| {
            b.intern_literal(RdfLiteral::simple(format!("{v:06}")))
                .expect("the fixture literal interns")
        })
        .collect();
    Terms {
        rdf_type,
        score,
        weight,
        count,
        price,
        label,
        status,
        parent,
        link,
        classes,
        statuses,
        graphs,
        subjects,
        floats,
        doubles,
        decimals,
        integers,
        labels,
    }
}

/// Push one quad if the budget allows; `false` when it is spent.
fn emit(
    b: &mut RdfDatasetBuilder,
    left: &mut usize,
    row: (TermId, TermId, TermId, Option<TermId>),
) -> bool {
    if *left == 0 {
        return false;
    }
    *left -= 1;
    b.push_quad(row.0, row.1, row.2, row.3)
        .expect("the fixture quad pushes");
    true
}

/// The builder holding exactly `layout.quads` distinct quads.
pub(crate) fn builder(layout: &Layout) -> RdfDatasetBuilder {
    let mut b = RdfDatasetBuilder::new();
    let t = intern_terms(&mut b, layout);
    let subjects = layout.subjects;

    let mut left = layout.default_quads;
    'default: for i in 0..subjects {
        let d = draws(i, layout.span);
        let s = t.subjects[i];
        let v = d.value as usize;
        let mut rows = vec![
            (s, t.rdf_type, t.classes[d.class]),
            (s, t.score, t.floats[v]),
            (s, t.weight, t.doubles[v]),
            (s, t.count, t.integers[v / 8]),
            (s, t.price, t.decimals[v]),
            (s, t.label, t.labels[v / 8]),
            (s, t.status, t.statuses[d.status as usize]),
        ];
        if i > 0 {
            rows.push((s, t.parent, t.subjects[(i - 1) / 3]));
        }
        for j in 0..=(i % 7) {
            rows.push((s, t.link, t.subjects[(i * 7 + j * 31 + 1) % subjects]));
        }
        for (s, p, o) in rows {
            if !emit(&mut b, &mut left, (s, p, o, None)) {
                break 'default;
            }
        }
    }
    assert_eq!(left, 0, "the default graph quota is spent");

    // A stride coprime to the subject count keeps a graph's subjects distinct.
    let step = (13..)
        .step_by(2)
        .find(|step| gcd(*step, subjects) == 1)
        .expect("a coprime stride exists");
    for (k, graph) in t.graphs.iter().enumerate() {
        let mut left =
            layout.named_quads / NAMED_GRAPHS + usize::from(k < layout.named_quads % NAMED_GRAPHS);
        'graph: for j in 0.. {
            let i = (k * 101 + j * step) % subjects;
            let d = draws(i, layout.span);
            let s = t.subjects[i];
            let mut rows = vec![
                (s, t.rdf_type, t.classes[d.class]),
                (s, t.score, t.floats[d.value as usize]),
                (s, t.status, t.statuses[d.status as usize]),
            ];
            for l in 0..=(j % 2) {
                rows.push((s, t.link, t.subjects[(i * 7 + l * 31 + 1) % subjects]));
            }
            for (s, p, o) in rows {
                if !emit(&mut b, &mut left, (s, p, o, Some(*graph))) {
                    break 'graph;
                }
            }
        }
    }
    b
}

const fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Old tree: ordinary freeze only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Data {
    Ordinary,
}
pub(crate) fn freeze(layout: &Layout, _how: Data) -> Arc<RdfDataset> {
    builder(layout).freeze().expect("freezes")
}
fn prologue() -> String {
    format!("PREFIX ex: <{NS}>\nPREFIX xsd: <{XSD_NS}>\n")
}

/// The datatype of the constant a numeric filter compares with, and the property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Numeric {
    Float,
    Double,
    Integer,
    Decimal,
    Text,
    /// The float property against an integer constant.
    FloatVsInteger,
    /// The float property against a decimal constant.
    FloatVsDecimal,
}

impl Numeric {
    const ALL: [Self; 7] = [
        Self::Float,
        Self::Double,
        Self::Integer,
        Self::Decimal,
        Self::Text,
        Self::FloatVsInteger,
        Self::FloatVsDecimal,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Double => "double",
            Self::Integer => "integer",
            Self::Decimal => "decimal",
            Self::Text => "string",
            Self::FloatVsInteger => "float_vs_integer",
            Self::FloatVsDecimal => "float_vs_decimal",
        }
    }

    /// The plain-FILTER query over every subject, with this property and constant.
    fn query(self, layout: &Layout) -> String {
        let t = layout.threshold();
        let (property, constant) = match self {
            Self::Float => ("score", format!("\"{t}.0\"^^xsd:float")),
            Self::Double => ("weight", format!("\"{t}.0E0\"^^xsd:double")),
            Self::Integer => ("count", t.to_string()),
            Self::Decimal => ("price", format!("{t}.0")),
            Self::Text => ("label", format!("\"{t:06}\"")),
            Self::FloatVsInteger => ("score", t.to_string()),
            Self::FloatVsDecimal => ("score", format!("{t}.0")),
        };
        format!(
            "{}SELECT ?x ?v WHERE {{ ?x ex:status ?st . ?x ex:{property} ?v . FILTER(?v < {constant}) }}",
            prologue()
        )
    }
}

/// One of the reported query shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Query {
    /// NOT EXISTS, GROUP BY, HAVING and COUNT over named graphs.
    Q1,
    /// FILTER and a property path.
    Q2,
    /// NOT EXISTS, GROUP BY and a sub-SELECT.
    Q3,
    /// Plain FILTER on a float.
    Q4,
    /// NOT EXISTS with FILTER.
    Q5,
    /// One triple pattern: never plans.
    Control,
    /// Plain FILTER with the given property and constant datatype.
    Numeric(Numeric),
}

impl Query {
    pub(crate) fn label(self) -> String {
        match self {
            Self::Q1 => "q1_not_exists_group_having".into(),
            Self::Q2 => "q2_filter_path".into(),
            Self::Q3 => "q3_not_exists_group_subquery".into(),
            Self::Q4 => "q4_filter".into(),
            Self::Q5 => "q5_not_exists_filter".into(),
            Self::Control => "control_one_pattern".into(),
            Self::Numeric(kind) => format!("numeric_{}", kind.label()),
        }
    }

    /// The rows of every variant of one query form one equivalence group.
    fn group(self) -> String {
        match self {
            Self::Numeric(_) => "numeric".into(),
            other => other.label(),
        }
    }

    fn text(self, layout: &Layout) -> String {
        let t = layout.threshold();
        let body = match self {
            Self::Q1 => "SELECT ?g (COUNT(?x) AS ?n) WHERE { \
                 GRAPH ?g { ?x a ex:C0 ; ex:score ?s ; ex:status ?st } \
                 FILTER NOT EXISTS { ?x ex:status ex:closed } \
                 } GROUP BY ?g HAVING (COUNT(?x) > 1)"
                .to_owned(),
            Self::Q2 => format!(
                "SELECT ?x ?y ?s WHERE {{ ?x ex:parent+ ?y . ?y a ex:C2 . ?x ex:score ?s . \
                 FILTER(?s > \"{t}.0\"^^xsd:float) }}"
            ),
            Self::Q3 => format!(
                "SELECT ?c (COUNT(?x) AS ?n) WHERE {{ \
                 {{ SELECT ?x ?c WHERE {{ GRAPH ?g {{ ?x a ?c ; ex:status ?st }} \
                 ?x ex:count ?k . FILTER(?k > {}) }} }} \
                 FILTER NOT EXISTS {{ ?x ex:status ex:closed }} \
                 }} GROUP BY ?c",
                t / 2
            ),
            Self::Q4 => return Numeric::Float.query(layout),
            Self::Q5 => "SELECT ?x ?s WHERE { ?x ex:score ?s ; ex:status ?st . \
                 FILTER NOT EXISTS { ?x ex:parent ?p . ?p ex:score ?ps . FILTER(?ps > ?s) } }"
                .to_owned(),
            Self::Control => "SELECT ?x ?v WHERE { ?x ex:score ?v }".to_owned(),
            Self::Numeric(kind) => return kind.query(layout),
        };
        format!("{}{body}", prologue())
    }
}

/// Governors on the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Governance {
    None,
    Metered,
}

/// Worker threads on the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Threads {
    /// The global rayon pool.
    All,
    /// `EvalOptions::force_sequential`.
    ForceSequential,
}

impl Threads {
    const fn label(self) -> &'static str {
        match self {
            Self::All => "all_threads",
            Self::ForceSequential => "force_sequential",
        }
    }
}

/// A SHACL shapes graph: which targets it declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeSet {
    /// Three `sh:SPARQLTarget` class selections.
    SparqlClass,
    /// Two `sh:SPARQLTarget` property selections.
    SparqlProperty,
    /// Three `sh:SPARQLTarget` class selections with a FILTER.
    SparqlFilter,
    /// The three class selections as `sh:targetClass`: no SPARQL.
    NativeClass,
    /// Every one of the above, with one `sh:sparql` constraint added.
    All,
}

impl ShapeSet {
    const BIND: [Self; 5] = [
        Self::SparqlClass,
        Self::SparqlProperty,
        Self::SparqlFilter,
        Self::NativeClass,
        Self::All,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::SparqlClass => "sparql_class_targets",
            Self::SparqlProperty => "sparql_property_targets",
            Self::SparqlFilter => "sparql_filter_targets",
            Self::NativeClass => "native_class_targets",
            Self::All => "all_shapes",
        }
    }

    fn turtle(self, layout: &Layout) -> String {
        let high = layout.high_count();
        let t = layout.threshold();
        let mut out = format!(
            "@prefix sh: <{}> .\n@prefix ex: <{NS}> .\n@prefix xsd: <{XSD_NS}> .\n\n",
            sh::NS
        );
        let constraints = format!(
            "    sh:property [ sh:path ex:score ; sh:datatype xsd:float ; sh:minCount 1 ] ;\n    \
             sh:property [ sh:path ex:count ; sh:maxInclusive {high} ] ."
        );
        let sparql = matches!(
            self,
            Self::SparqlClass | Self::SparqlProperty | Self::SparqlFilter | Self::All
        );
        let native = matches!(self, Self::NativeClass | Self::All);
        let class = matches!(self, Self::SparqlClass | Self::All);
        let property = matches!(self, Self::SparqlProperty | Self::All);
        let filter = matches!(self, Self::SparqlFilter | Self::All);
        let _ = sparql;
        if class {
            for c in 0..3 {
                out += &format!(
                    "ex:SparqlClass{c} a sh:NodeShape ;\n    sh:target [ a sh:SPARQLTarget ; \
                     sh:select \"SELECT ?this WHERE {{ ?this a <{NS}C{c}> }}\" ] ;\n{constraints}\n\n"
                );
            }
        }
        if property {
            for (n, predicate) in ["link", "parent"].iter().enumerate() {
                out += &format!(
                    "ex:SparqlProperty{n} a sh:NodeShape ;\n    sh:target [ a sh:SPARQLTarget ; \
                     sh:select \"SELECT ?this WHERE {{ ?this <{NS}{predicate}> ?o }}\" ] ;\n    \
                     sh:property [ sh:path ex:status ; sh:minCount 1 ; sh:maxCount 1 ] .\n\n"
                );
            }
        }
        if filter {
            for c in 1..4 {
                out += &format!(
                    "ex:SparqlFilter{c} a sh:NodeShape ;\n    sh:target [ a sh:SPARQLTarget ; \
                     sh:select \"\"\"SELECT ?this WHERE {{ ?this a <{NS}C{c}> ; <{NS}score> ?s . \
                     FILTER(?s > \"{t}.0\"^^<{XSD_FLOAT}>) }}\"\"\" ] ;\n{constraints}\n\n"
                );
            }
        }
        if native {
            for c in 0..3 {
                out += &format!(
                    "ex:NativeClass{c} a sh:NodeShape ;\n    sh:targetClass ex:C{c} ;\n{constraints}\n\n"
                );
            }
        }
        if self == Self::All {
            out += &format!(
                "ex:PerFocusSparql a sh:NodeShape ;\n    sh:targetClass ex:C2 ;\n    \
                 sh:sparql [ a sh:SPARQLConstraint ; sh:message \"count too high\" ; \
                 sh:select \"\"\"SELECT $this ?value WHERE {{ $this <{NS}count> ?value . \
                 FILTER(?value > {high}) }}\"\"\" ] .\n"
            );
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Old-tree rows: plain Instant timing, no harness, no counters.
// ---------------------------------------------------------------------------

fn execute(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    prepared: &PreparedQuery,
    governance: Governance,
) -> usize {
    let result = match governance {
        Governance::None => engine
            .query_prepared(dataset, prepared, &[], QueryOptions::EMPTY)
            .expect("the query completes"),
        Governance::Metered => match engine
            .query_prepared_governed_view(
                &**dataset,
                prepared,
                &[],
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .expect("the governed query completes")
        {
            GovernedOutcome::Complete { result, .. } => result,
            _ => panic!("the metered ceilings are unreachable"),
        },
    };
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT")
    };
    rows.len()
}

fn engine(threads: Threads) -> NativeSparqlEngine {
    if threads == Threads::ForceSequential {
        NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential: true,
            ..EvalOptions::default()
        })
    } else {
        NativeSparqlEngine::new()
    }
}

fn median(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (v[v.len() / 2], v[0], v[v.len() - 1])
}

fn report(quads: usize, id: &str, samples: Vec<f64>, witness: usize) {
    let (m, lo, hi) = median(samples);
    println!("old {quads} {id} median_ms={m:.3} min_ms={lo:.3} max_ms={hi:.3} witness={witness}");
}

/// Every row of the old tree can express, as (id, setup + run) pairs. `setup` is
/// built per sample and its output run once; `warm` rows run the same setup output
/// repeatedly after one unmeasured run.
fn run_rows(quads: usize, only: Option<&str>, child_phase: Option<&str>) {
    let layout = Layout::new(quads);
    let shared = freeze(&layout, Data::Ordinary);
    assert_eq!(shared.quad_count(), layout.quads);
    let all = [Query::Q1, Query::Q2, Query::Q3, Query::Q4, Query::Q5];
    // Once-per-dataset costs on the shared dataset, as in the new bench.
    let warmer = NativeSparqlEngine::new();
    for q in all {
        let p = warmer.prepare_query(&q.text(&layout), None).unwrap();
        execute(&warmer, &shared, &p, Governance::None);
    }
    let want = |id: &str| only.map_or(true, |o| o == id);
    let samples = 7;

    // freeze
    if want("freeze/ordinary_freeze") {
        let mut t = Vec::new();
        let mut w = 0;
        for _ in 0..samples {
            let b = builder(&layout);
            if child_phase == Some("setup") {
                break;
            }
            let s = Instant::now();
            let d = b.freeze().expect("freezes");
            t.push(s.elapsed().as_secs_f64() * 1e3);
            w = d.quad_count();
            std::mem::forget(d);
            if child_phase.is_some() {
                break;
            }
        }
        if child_phase.is_none() {
            report(quads, "freeze/ordinary_freeze", t, w);
        }
    }

    // SHACL bind and validate
    for shapes in [
        ShapeSet::SparqlClass,
        ShapeSet::SparqlProperty,
        ShapeSet::SparqlFilter,
        ShapeSet::NativeClass,
        ShapeSet::All,
    ] {
        let id = format!("shacl_bind/{}", shapes.label());
        if !want(&id) {
            continue;
        }
        let mut t = Vec::new();
        let mut w = 0;
        for _ in 0..samples {
            let d = freeze(&layout, Data::Ordinary);
            let prepared = PreparedShapes::new(Arc::new(
                parse_shapes(&shapes.turtle(&layout), None).unwrap(),
            ));
            if child_phase == Some("setup") {
                break;
            }
            let s = Instant::now();
            let v = prepared.bind_shared_dataset(Arc::clone(&d)).unwrap();
            t.push(s.elapsed().as_secs_f64() * 1e3);
            w = v.validate().unwrap().results.len() + 1;
            std::mem::forget(v);
            if child_phase.is_some() {
                break;
            }
        }
        if child_phase.is_none() {
            report(quads, &id, t, w);
        }
    }
    if want("shacl_validate/all_shapes") {
        let mut t = Vec::new();
        let mut w = 0;
        for _ in 0..samples {
            let prepared = PreparedShapes::new(Arc::new(
                parse_shapes(&ShapeSet::All.turtle(&layout), None).unwrap(),
            ));
            let v = prepared.bind_shared_dataset(Arc::clone(&shared)).unwrap();
            if child_phase == Some("setup") {
                break;
            }
            let s = Instant::now();
            let r = v.validate().unwrap();
            t.push(s.elapsed().as_secs_f64() * 1e3);
            w = r.results.len();
            if child_phase.is_some() {
                break;
            }
        }
        if child_phase.is_none() {
            report(quads, "shacl_validate/all_shapes", t, w);
        }
    }

    // SELECT rows
    let mut selects: Vec<(String, Query, Governance, Threads, bool, bool)> = Vec::new();
    for q in all {
        for warm in [false, true] {
            selects.push((
                format!(
                    "select/{}/{}/ordinary_freeze",
                    q.label(),
                    if warm { "warm" } else { "cold" }
                ),
                q,
                Governance::None,
                Threads::All,
                warm,
                false,
            ));
        }
    }
    selects.push((
        format!(
            "select/{}/first_on_dataset/ordinary_freeze",
            Query::Q4.label()
        ),
        Query::Q4,
        Governance::None,
        Threads::All,
        false,
        true,
    ));
    for warm in [false, true] {
        selects.push((
            format!(
                "select/{}/{}/ordinary_freeze",
                Query::Control.label(),
                if warm { "warm" } else { "cold" }
            ),
            Query::Control,
            Governance::None,
            Threads::All,
            warm,
            false,
        ));
    }
    for k in Numeric::ALL {
        for warm in [false, true] {
            selects.push((
                format!(
                    "select/{}/{}/ordinary_freeze",
                    Query::Numeric(k).label(),
                    if warm { "warm" } else { "cold" }
                ),
                Query::Numeric(k),
                Governance::None,
                Threads::All,
                warm,
                false,
            ));
        }
    }
    for q in [Query::Q1, Query::Q4, Query::Q5] {
        for warm in [false, true] {
            selects.push((
                format!(
                    "select/{}/{}/metered",
                    q.label(),
                    if warm { "warm" } else { "cold" }
                ),
                q,
                Governance::Metered,
                Threads::All,
                warm,
                false,
            ));
        }
        selects.push((
            format!("select/{}/warm/force_sequential", q.label()),
            q,
            Governance::None,
            Threads::ForceSequential,
            true,
            false,
        ));
    }
    for (id, q, gov, threads, warm, fresh) in selects {
        if !want(&id) {
            continue;
        }
        let mut t = Vec::new();
        let mut w = 0;
        let text = q.text(&layout);
        if warm {
            let e = engine(threads);
            let p = e.prepare_query(&text, None).unwrap();
            execute(&e, &shared, &p, gov);
            if child_phase == Some("setup") {
                continue;
            }
            for _ in 0..samples {
                let s = Instant::now();
                w = execute(&e, &shared, &p, gov);
                t.push(s.elapsed().as_secs_f64() * 1e3);
                if child_phase.is_some() {
                    break;
                }
            }
        } else {
            for _ in 0..samples {
                let d = if fresh {
                    freeze(&layout, Data::Ordinary)
                } else {
                    Arc::clone(&shared)
                };
                let e = engine(threads);
                let p = e.prepare_query(&text, None).unwrap();
                if child_phase == Some("setup") {
                    break;
                }
                let s = Instant::now();
                w = execute(&e, &d, &p, gov);
                t.push(s.elapsed().as_secs_f64() * 1e3);
                if child_phase.is_some() {
                    break;
                }
            }
        }
        if child_phase.is_none() {
            report(quads, &id, t, w);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--child") {
        // --child <quads> <row id> <setup|run>
        run_rows(args[2].parse().unwrap(), Some(&args[3]), Some(&args[4]));
        return;
    }
    for quads in sizes("PURRDF_FROZEN_EVAL_QUADS") {
        eprintln!("old port: {quads} quads");
        run_rows(
            quads,
            args.get(1)
                .map(String::as_str)
                .filter(|a| !a.starts_with("--")),
            None,
        );
    }
}
