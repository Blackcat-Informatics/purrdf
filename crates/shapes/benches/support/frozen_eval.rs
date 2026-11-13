// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The frozen dataset, shapes graph, query set and row catalog the
//! `frozen_eval_counts` bench reads. Every row is defined once here as a [`Row`]
//! and turned into a runnable [`Op`] by [`op`].
//!
//! # The workload
//!
//! A frozen dataset with a default graph (47 percent of the quads) and
//! [`NAMED_GRAPHS`] named graphs (the other 53 percent, equal in size), shaped
//! like the load a user reported: typed entities with an `rdf:type`, a float,
//! a double, an integer, a decimal and a string property, a status, a parent
//! tree about eight deep and a many-valued link, all under `example.org`.
//! The named graphs repeat the type, the score and the status of subjects that
//! live in the default graph, so GROUP BY and NOT EXISTS cross graphs.
//!
//! All five numeric properties derive from one draw `v` below [`Layout::span`]:
//! the float, the double and the decimal hold `v / 8` exactly, the integer holds
//! `v / 8` rounded down and the string holds that integer zero-padded. A filter
//! "below `t`" with `t = span / 16` therefore keeps exactly the same subjects
//! whichever datatype carries the constant, which is what lets the numeric rows
//! be compared as the same work.
//!
//! Nothing here reads a clock, the filesystem or an unseeded source of
//! randomness; every draw comes from the test kit's seeded generator.

use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};

use purrdf_core::{DatasetView, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult, TermId};
use purrdf_iri::vocab::{rdf, sh};
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_shapes::engine::{PreparedShapes, PreparedValidator, parse_shapes};
use purrdf_sparql_eval::{NativeSparqlEngine, PreparedQuery, QueryOptions};
use purrdf_testkit::rng::SplitMix64;
use purrdf_xsd::datatype::{XSD_DECIMAL, XSD_DOUBLE, XSD_FLOAT, XSD_INTEGER, XSD_NS};

/// The fixture namespace. Caller-supplied, and `example.org` by rule.
pub(crate) const NS: &str = "https://example.org/ns/";

/// The number of named graphs, as in the reported dataset.
pub(crate) const NAMED_GRAPHS: usize = 117;

/// The routine size: one thousandth of the reported load.
pub(crate) const ROUTINE_QUADS: [usize; 1] = [22_000];

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
    let mut rng =
        SplitMix64::new(SEED ^ (subject as u64).wrapping_mul(purrdf_hash::mix::GOLDEN_GAMMA));
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

/// How a dataset is frozen, or fronted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Data {
    /// `RdfDatasetBuilder::freeze`: statistics are built on first use.
    Ordinary,
    /// `RdfDatasetBuilder::freeze_with_memory`: statistics are built in the freeze.
    Eager,
}

impl Data {
    const fn label(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary_freeze",
            Self::Eager => "freeze_with_memory",
        }
    }
}

/// Freeze a built builder the way `how` names.
///
/// `freeze_with_memory` refunds the builder's buffers from the account, so the
/// live total handed to it must cover them. The resident account admits every
/// total and keeps none, so a generous total is an accounting convenience, not a
/// limit.
pub(crate) fn freeze_builder(b: RdfDatasetBuilder, how: Data) -> Arc<RdfDataset> {
    match how {
        Data::Ordinary => b.freeze().expect("the fixture freezes"),
        Data::Eager => {
            let mut resident = Resident;
            let mut memory = Memory::resume(&mut resident, 1 << 40);
            Arc::new(
                b.freeze_with_memory(&mut memory)
                    .expect("the fixture freezes with memory"),
            )
        }
    }
}

/// Freeze a fresh dataset; the builder construction is part of the cost.
pub(crate) fn freeze(layout: &Layout, how: Data) -> Arc<RdfDataset> {
    freeze_builder(builder(layout), how)
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

fn prologue() -> String {
    format!("PREFIX ex: <{NS}>\nPREFIX xsd: <{XSD_NS}>\n")
}

/// The datatype of the constant a numeric filter compares with, and the property.
/// The plain-FILTER query over every subject on the float property.
fn float_filter_query(layout: &Layout) -> String {
    let t = layout.threshold();
    format!(
        "{}SELECT ?x ?v WHERE {{ ?x ex:status ?st . ?x ex:score ?v . \
         FILTER(?v < \"{t}.0\"^^xsd:float) }}",
        prologue()
    )
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
}

impl Query {
    pub(crate) fn label(self) -> String {
        match self {
            Self::Q1 => "q1_not_exists_group_having".into(),
            Self::Q2 => "q2_filter_path".into(),
            Self::Q3 => "q3_not_exists_group_subquery".into(),
            Self::Q4 => "q4_filter".into(),
            Self::Q5 => "q5_not_exists_filter".into(),
        }
    }

    /// The rows of every variant of one query form one equivalence group.
    fn group(self) -> String {
        self.label()
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
            Self::Q4 => return float_filter_query(layout),
            Self::Q5 => "SELECT ?x ?s WHERE { ?x ex:score ?s ; ex:status ?st . \
                 FILTER NOT EXISTS { ?x ex:parent ?p . ?p ex:score ?ps . FILTER(?ps > ?s) } }"
                .to_owned(),
        };
        format!("{}{body}", prologue())
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
                write!(
                    out,
                    "ex:SparqlClass{c} a sh:NodeShape ;\n    sh:target [ a sh:SPARQLTarget ; \
                     sh:select \"SELECT ?this WHERE {{ ?this a <{NS}C{c}> }}\" ] ;\n{constraints}\n\n"
                ).expect("a String takes the text");
            }
        }
        if property {
            for (n, predicate) in ["link", "parent"].iter().enumerate() {
                write!(
                    out,
                    "ex:SparqlProperty{n} a sh:NodeShape ;\n    sh:target [ a sh:SPARQLTarget ; \
                     sh:select \"SELECT ?this WHERE {{ ?this <{NS}{predicate}> ?o }}\" ] ;\n    \
                     sh:property [ sh:path ex:status ; sh:minCount 1 ; sh:maxCount 1 ] .\n\n"
                )
                .expect("a String takes the text");
            }
        }
        if filter {
            for c in 1..4 {
                write!(
                    out,
                    "ex:SparqlFilter{c} a sh:NodeShape ;\n    sh:target [ a sh:SPARQLTarget ; \
                     sh:select \"\"\"SELECT ?this WHERE {{ ?this a <{NS}C{c}> ; <{NS}score> ?s . \
                     FILTER(?s > \"{t}.0\"^^<{XSD_FLOAT}>) }}\"\"\" ] ;\n{constraints}\n\n"
                )
                .expect("a String takes the text");
            }
        }
        if native {
            for c in 0..3 {
                write!(
                    out,
                    "ex:NativeClass{c} a sh:NodeShape ;\n    sh:targetClass ex:C{c} ;\n{constraints}\n\n"
                ).expect("a String takes the text");
            }
        }
        if self == Self::All {
            write!(
                out,
                "ex:PerFocusSparql a sh:NodeShape ;\n    sh:targetClass ex:C2 ;\n    \
                 sh:sparql [ a sh:SPARQLConstraint ; sh:message \"count too high\" ; \
                 sh:select \"\"\"SELECT $this ?value WHERE {{ $this <{NS}count> ?value . \
                 FILTER(?value > {high}) }}\"\"\" ] .\n"
            )
            .expect("a String takes the text");
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Rows
// ---------------------------------------------------------------------------

/// What one row measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Spec {
    /// Builder to frozen dataset, builder construction outside the counted window.
    Freeze(Data),
    /// A `snapshot_identity()` call on a freshly frozen dataset.
    IdentityFirst(Data),
    /// A second call on the same dataset.
    IdentitySecond,
    /// A SELECT.
    Select {
        query: Query,
        data: Data,
        warm: bool,
        /// The dataset is frozen inside the sample, so identity and every
        /// once-per-dataset cost lands in the first query.
        fresh_dataset: bool,
    },
    /// A SHACL bind on a freshly frozen dataset.
    Bind {
        shapes: ShapeSet,
        identity_warm: bool,
        data: Data,
    },
    /// Validation of a bound validator: the first `validate()` on that bind.
    Validate { data: Data },
}

impl Spec {
    /// A row whose measured call may be repeated on one prepared state: a warm
    /// SELECT, a second identity call, and the first call on a dataset frozen with
    /// eager statistics, which sealed its identity in the freeze.
    pub(crate) const fn repeatable(self) -> bool {
        matches!(
            self,
            Self::Select { warm: true, .. }
                | Self::IdentitySecond
                | Self::IdentityFirst(Data::Eager)
        )
    }
}

/// One bench row: a stable id and the group whose members must agree.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub id: String,
    pub group: String,
    pub spec: Spec,
}

/// Every row of one size, in report order.
pub(crate) fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    let mut add = |id: String, group: &str, spec: Spec| {
        rows.push(Row {
            id,
            group: group.to_owned(),
            spec,
        });
    };
    for data in [Data::Ordinary, Data::Eager] {
        add(
            format!("freeze/{}", data.label()),
            "freeze",
            Spec::Freeze(data),
        );
        add(
            format!("identity/first_call/{}", data.label()),
            "identity",
            Spec::IdentityFirst(data),
        );
    }
    add(
        "identity/second_call/ordinary_freeze".into(),
        "identity",
        Spec::IdentitySecond,
    );

    for shapes in ShapeSet::BIND {
        for identity_warm in [false, true] {
            add(
                format!(
                    "shacl_bind/{}/identity_{}",
                    shapes.label(),
                    if identity_warm { "warmed" } else { "cold" }
                ),
                &format!("bind_{}", shapes.label()),
                Spec::Bind {
                    shapes,
                    identity_warm,
                    data: Data::Ordinary,
                },
            );
        }
    }
    add(
        "shacl_bind/all_shapes/freeze_with_memory".into(),
        "bind_all_shapes",
        Spec::Bind {
            shapes: ShapeSet::All,
            identity_warm: true,
            data: Data::Eager,
        },
    );
    for data in [Data::Ordinary, Data::Eager] {
        add(
            format!("shacl_validate/all_shapes/{}", data.label()),
            "validate",
            Spec::Validate { data },
        );
    }

    let reported = [Query::Q1, Query::Q2, Query::Q3, Query::Q4, Query::Q5];
    // Cold: a fresh engine plans on a dataset whose identity is sealed. Warm: the
    // same engine runs the prepared query again.
    for query in reported {
        for data in [Data::Ordinary, Data::Eager] {
            for warm in [false, true] {
                add(
                    format!(
                        "select/{}/{}/{}",
                        query.label(),
                        if warm { "warm" } else { "cold" },
                        data.label()
                    ),
                    &query.group(),
                    Spec::Select {
                        query,
                        data,
                        warm,
                        fresh_dataset: false,
                    },
                );
            }
        }
    }
    // The first query on a freshly frozen dataset: every once-per-dataset cost
    // lands in it.
    for query in [Query::Q1, Query::Q3, Query::Q4] {
        for data in [Data::Ordinary, Data::Eager] {
            add(
                format!("select/{}/first_on_dataset/{}", query.label(), data.label()),
                &query.group(),
                Spec::Select {
                    query,
                    data,
                    warm: false,
                    fresh_dataset: true,
                },
            );
        }
    }
    rows
}

// ---------------------------------------------------------------------------
// Runnable rows
// ---------------------------------------------------------------------------

/// How much a run reports about its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Detail {
    /// The solution count only, cheap enough to sit inside the counted window.
    Count,
    /// Also a digest of every solution, for the equality check.
    Digest,
}

/// What a run produced: compared across the rows of one group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Witness {
    pub count: u64,
    pub digest: u64,
}

/// One prepared row. Calling it runs the measured work; a cold row is built per
/// sample and called once, a warm row is built once and called repeatedly.
pub(crate) struct Op {
    run: Box<dyn FnMut(Detail) -> Witness>,
}

impl Op {
    pub(crate) fn run(&mut self, detail: Detail) -> Witness {
        (self.run)(detail)
    }
}

/// The datasets shared by the rows that do not freeze their own.
pub(crate) struct Fixtures {
    pub layout: Layout,
    ordinary: OnceLock<Arc<RdfDataset>>,
    eager: OnceLock<Arc<RdfDataset>>,
}

impl Fixtures {
    pub(crate) fn new(quads: usize) -> Self {
        Self {
            layout: Layout::new(quads),
            ordinary: OnceLock::new(),
            eager: OnceLock::new(),
        }
    }

    /// The shared dataset of `data`, with its once-per-dataset costs already paid:
    /// every query form has run on it, so a cold row measures planning on a fresh
    /// engine and not the dataset's first use.
    pub(crate) fn shared(&self, data: Data) -> Arc<RdfDataset> {
        let (cell, how) = match data {
            Data::Ordinary => (&self.ordinary, Data::Ordinary),
            Data::Eager => (&self.eager, Data::Eager),
        };
        Arc::clone(cell.get_or_init(|| {
            let dataset = freeze(&self.layout, how);
            assert_eq!(
                dataset.quad_count(),
                self.layout.quads,
                "the layout is exact"
            );
            let _ = DatasetView::snapshot_identity(&*dataset);
            let engine = NativeSparqlEngine::new();
            for query in [Query::Q1, Query::Q2, Query::Q3, Query::Q4, Query::Q5] {
                let prepared = engine
                    .prepare_query(&query.text(&self.layout), None)
                    .expect("the query prepares");
                let _ = execute(&engine, &dataset, &prepared);
            }
            dataset
        }))
    }
}

fn execute(
    engine: &NativeSparqlEngine,
    dataset: &RdfDataset,
    prepared: &PreparedQuery,
) -> SparqlResult {
    engine
        .query_prepared_view(dataset, prepared, &[], &QueryOptions::EMPTY)
        .expect("the query completes")
}

fn witness(result: &SparqlResult, detail: Detail) -> Witness {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("the row is a SELECT");
    };
    let digest = if detail == Detail::Digest {
        let mut lines: Vec<String> = rows.iter().map(|row| format!("{row:?}")).collect();
        lines.sort_unstable();
        let mut hash = purrdf_hash::fnv::fnv1a64(b"");
        for line in &lines {
            hash = hash.rotate_left(5) ^ purrdf_hash::fnv::fnv1a64(line.as_bytes());
        }
        hash
    } else {
        0
    };
    Witness {
        count: rows.len() as u64,
        digest,
    }
}

fn digest64(identity: Option<purrdf_hash::hex::Digest32>) -> u64 {
    let identity = identity.expect("an RdfDataset reports an identity");
    u64::from_le_bytes(identity.as_bytes()[..8].try_into().expect("32 bytes"))
}

/// Turn a row into a runnable [`Op`]; the setup it does is outside the counted window.
pub(crate) fn op(row: &Row, fx: &Fixtures) -> Op {
    let layout = fx.layout;
    match row.spec {
        Spec::Freeze(how) => {
            let builder = builder(&layout);
            let mut builder = Some(builder);
            Op {
                run: Box::new(move |_| {
                    let b = builder.take().expect("a freeze row runs once per sample");
                    let dataset = freeze_builder(b, how);
                    let count = dataset.quad_count() as u64;
                    std::mem::forget(dataset);
                    Witness {
                        count,
                        digest: count,
                    }
                }),
            }
        }
        Spec::IdentityFirst(how) => {
            let dataset = freeze(&layout, how);
            Op {
                run: Box::new(move |_| {
                    let digest =
                        digest64(DatasetView::snapshot_identity(&*dataset).expect("reads"));
                    Witness { count: 1, digest }
                }),
            }
        }
        Spec::IdentitySecond => {
            let dataset = freeze(&layout, Data::Ordinary);
            let _ = DatasetView::snapshot_identity(&*dataset);
            Op {
                run: Box::new(move |_| {
                    let digest =
                        digest64(DatasetView::snapshot_identity(&*dataset).expect("reads"));
                    Witness { count: 1, digest }
                }),
            }
        }
        Spec::Select {
            query,
            data,
            warm: _,
            fresh_dataset,
        } => {
            let dataset = if fresh_dataset {
                freeze(&layout, data)
            } else {
                fx.shared(data)
            };
            let engine = NativeSparqlEngine::new();
            let prepared = engine
                .prepare_query(&query.text(&layout), None)
                .expect("the query prepares");
            Op {
                run: Box::new(move |detail| {
                    witness(&execute(&engine, &dataset, &prepared), detail)
                }),
            }
        }
        Spec::Bind {
            shapes,
            identity_warm,
            data,
        } => {
            let dataset = freeze(&layout, data);
            if identity_warm {
                let _ = DatasetView::snapshot_identity(&*dataset);
            }
            let prepared = prepared_shapes(shapes, &layout);
            Op {
                run: Box::new(move |detail| {
                    let validator = prepared
                        .bind_shared_dataset(Arc::clone(&dataset))
                        .expect("the data binds");
                    // A bind exposes no focus-node count, so the equality check
                    // validates once, outside the counted window, and compares the
                    // results the resolved targets lead to.
                    let witness = if detail == Detail::Digest {
                        let report = validator.validate().expect("validation completes");
                        let count = 1 + report.results.len() as u64;
                        Witness {
                            count,
                            digest: count * 2 + u64::from(report.conforms),
                        }
                    } else {
                        Witness {
                            count: 1,
                            digest: 0,
                        }
                    };
                    // Teardown is not part of the bind.
                    std::mem::forget(validator);
                    witness
                }),
            }
        }
        Spec::Validate { data } => {
            let dataset = fx.shared(data);
            let prepared = prepared_shapes(ShapeSet::All, &layout);
            let validator: PreparedValidator = prepared
                .bind_shared_dataset(dataset)
                .expect("the data binds");
            Op {
                run: Box::new(move |_| {
                    let _keep = &prepared;
                    let report = validator.validate().expect("validation completes");
                    let count = report.results.len() as u64;
                    Witness {
                        count,
                        digest: count * 2 + u64::from(report.conforms),
                    }
                }),
            }
        }
    }
}

fn prepared_shapes(shapes: ShapeSet, layout: &Layout) -> PreparedShapes {
    let parsed = parse_shapes(&shapes.turtle(layout), None).expect("the shapes graph parses");
    PreparedShapes::new(Arc::new(parsed))
}

/// Run every row once with full digests and refuse a group whose members differ,
/// or a result that reached no solution, so a variant that silently does other
/// work cannot pass as faster. Returns each row's id with its witness.
pub(crate) fn verify(fx: &Fixtures, rows: &[Row]) -> Vec<(String, Witness)> {
    let mut seen: Vec<(String, Witness)> = Vec::new();
    let mut groups: Vec<(String, String, Witness)> = Vec::new();
    for row in rows {
        let witness = op(row, fx).run(Detail::Digest);
        assert!(witness.count > 0, "{} produced nothing", row.id);
        match groups.iter().find(|(group, ..)| *group == row.group) {
            Some((_, first, expected)) => assert_eq!(
                (witness.count, witness.digest),
                (expected.count, expected.digest),
                "{} disagrees with {first}",
                row.id
            ),
            None => groups.push((row.group.clone(), row.id.clone(), witness)),
        }
        seen.push((row.id.clone(), witness));
    }
    seen
}
