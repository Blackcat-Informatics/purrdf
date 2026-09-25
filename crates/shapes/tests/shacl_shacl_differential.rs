// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! THE SHACL-SHACL DIFFERENTIAL ORACLE: PurRDF refuses a shapes graph exactly when
//! the W3C's own shapes graph for shapes graphs says it is ill-formed.
//!
//! The W3C publishes `shacl-shacl.ttl` — "a SHACL shapes graph to validate SHACL
//! shapes graphs" — vendored at `crates/shapes/spec/shacl-shacl.ttl`. It is an
//! independent statement of SHACL's syntax rules, written by the Working Group
//! rather than by this engine. So it is an oracle for the loader's refusals:
//!
//! * every shapes graph of the three corpora (the first-party corpus, the W3C
//!   SHACL 1.0 suite and the W3C SHACL 1.2 suite's `sht:Validate` entries), and
//! * generated MUTANTS of every one both sides accept — a literal where an IRI is
//!   required, a non-integer count, a SHACL list where a single value is
//!   required, a non-boolean flag, and a misspelled `sh:` predicate —
//!
//! is loaded by PurRDF's parser AND validated, as DATA, against `shacl-shacl.ttl`
//! as SHAPES; PurRDF must refuse it if and only if that validation reports a
//! `sh:Violation`.
//!
//! # The two ledgers, and which one must be empty
//!
//! [`STRICTER_THAN_SHACL_SHACL`] names each rule by which PurRDF refuses a graph
//! `shacl-shacl.ttl` cannot flag — a syntax rule it does not express (an unknown
//! term), or a feature this engine does not evaluate and therefore refuses rather
//! than silently skipping — with the exact number of inputs it covers.
//!
//! [`SHACL_SHACL_BEHIND_THE_SPEC`] is the other direction — `shacl-shacl.ttl`
//! flags a graph PurRDF accepts — and every such difference is an under-refusal
//! bug unless the vendored `shacl-shacl.ttl` itself lags the SHACL 1.2 Core text.
//! Each entry quotes the specification sentence that makes the graph well-formed.
//! Nothing else may appear there.

mod shacl_corpora;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf::{RdfDataset, SerializeGraph, serialize_dataset};
use purrdf_shapes::engine::validate_dataset_with_shapes_graph;
use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::report::Severity;
use purrdf_shapes::shapes::{Shapes, from_dataset_with_config_and_graph};
use purrdf_shapes::text_ingest::parse_turtle_document;

use shacl_corpora::shacl12::{Body, shacl12_cases};
use shacl_corpora::{file_iri, first_party_box_role_vocab, first_party_cases, w3c_cases};

/// The W3C shapes graph for shapes graphs.
const SHACL_SHACL: &str = include_str!("../spec/shacl-shacl.ttl");

const SH: &str = "http://www.w3.org/ns/shacl#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

// ── The ledgers ───────────────────────────────────────────────────────────────

/// A rule by which PurRDF refuses what `shacl-shacl.ttl` cannot flag:
/// `(name, what the refusal message contains, inputs it covers, why)`.
type Stricter = (&'static str, &'static str, usize, &'static str);

const STRICTER_THAN_SHACL_SHACL: &[Stricter] = &[
    (
        "unknown-term",
        "which is not a term of SHACL 1.2, SHACL Advanced Features",
        236,
        "shacl-shacl.ttl checks the terms it knows and ignores the rest, so a misspelled \
         parameter (sh:minCont) passes it; PurRDF refuses a sh:/shnex: predicate the census \
         does not classify, because an unread parameter checks nothing",
    ),
    (
        "root-class-value",
        "rootClass> on shape",
        1,
        "SHACL 1.2 Core §7.9.4: \"The values of sh:rootClass in a shape are either IRIs or \
         blank nodes that are well-formed SHACL lists where all members are IRIs.\" — \
         shacl-shacl.ttl states no rule for sh:rootClass",
    ),
    (
        "reifier-annotation-value",
        "on the reifiers of shape",
        2,
        "a per-constraint reifier annotation's value has the rule its shape-level form has — \
         SHACL 1.2 Core: \"In SHACL Core, the only valid values for sh:deactivated are the \
         constant literal node expressions true and false\", \"Each value of sh:severity is an \
         IRI\" — and PurRDF refuses a reifier {| sh:deactivated \"yes\" |} or \
         {| sh:severity \"not an IRI\" |} as it refuses the same value on a shape; \
         shacl-shacl.ttl never looks at reifiers, so the mutants that rewrite the W3C suite's \
         own annotated cases (deactivated-003, severity-003) pass it",
    ),
    (
        "reification-required-datatype",
        "reificationRequired> on shape",
        6,
        "SHACL 1.2 Core: \"The values of sh:reificationRequired in a shape are literals with \
         datatype xsd:boolean\"; shacl-shacl.ttl states no rule for sh:reificationRequired",
    ),
    (
        "sparql-pre-binding",
        "Pre-binding of Variables in SPARQL Queries",
        12,
        "the pre-binding restrictions of SHACL 1.2 SPARQL Extensions, Appendix A (no MINUS, \
         VALUES or SERVICE, no assignment to a pre-bound variable, subqueries project it) are \
         rules about SPARQL text, which shacl-shacl.ttl does not parse",
    ),
    (
        "sparql-constraint-severity",
        "sh:severity on <",
        4,
        "a SPARQL-based constraint's sh:severity names a severity, an IRI, exactly as a shape's \
         does (SHACL 1.2 Core §3.6.2.4); shacl-shacl.ttl checks sh:severity on shapes only, and \
         the object of sh:sparql is not one",
    ),
];

/// A graph `shacl-shacl.ttl` flags that SHACL 1.2 Core makes well-formed:
/// `(name, the Violation results it covers as (component local name, result path,
/// source shape), inputs it covers, the SHACL 1.2 Core sentence)`. An empty result
/// path or source shape in the pattern matches an absent one.
type BehindTheSpec = (
    &'static str,
    &'static [(&'static str, &'static str, &'static str)],
    usize,
    &'static str,
);

const SHACL_SHACL_BEHIND_THE_SPEC: &[BehindTheSpec] = &[
    (
        "closed-by-types",
        &[(
            "DatatypeConstraintComponent",
            "<http://www.w3.org/ns/shacl#closed>",
            "",
        )],
        2,
        "SHACL 1.2 Core §7.9.1: \"The values of sh:closed in a shape are literals with \
         datatype xsd:boolean or the IRI sh:ByTypes.\" — shacl-shacl.ttl still requires an \
         xsd:boolean (closed-datatype), so the W3C suite's own closed-003 and closed-004 fail \
         it",
    ),
    (
        "list-valued-node-kind",
        &[(
            "InConstraintComponent",
            "<http://www.w3.org/ns/shacl#nodeKind>",
            "",
        )],
        1,
        "SHACL 1.2 Core §4.1.3: \"The value of sh:nodeKind in a shape is either an IRI or a \
         blank node that is a well-formed SHACL list where all members are IRIs.\" — \
         shacl-shacl.ttl still requires one of the six SHACL 1.0 node-kind IRIs",
    ),
    (
        "path-valued-property-pair",
        &[
            (
                "NodeKindConstraintComponent",
                "<http://www.w3.org/ns/shacl#equals>",
                "",
            ),
            (
                "NodeKindConstraintComponent",
                "<http://www.w3.org/ns/shacl#disjoint>",
                "",
            ),
            (
                "NodeKindConstraintComponent",
                "<http://www.w3.org/ns/shacl#lessThan>",
                "",
            ),
            (
                "NodeKindConstraintComponent",
                "<http://www.w3.org/ns/shacl#lessThanOrEquals>",
                "",
            ),
        ],
        4,
        "SHACL 1.2 Core §7.6.1: \"The values of sh:equals in a shape are well-formed SHACL \
         property paths.\" — and §7.6.2, §7.6.4 and §7.6.5 say the same of sh:disjoint, \
         sh:lessThan and sh:lessThanOrEquals. shacl-shacl.ttl still requires an IRI for all \
         four (equals-nodeKind, disjoint-nodeKind, lessThan-nodeKind, \
         lessThanOrEquals-nodeKind), so the W3C suite's own equals-002, disjoint-002, \
         lessThan-003 and lessThanOrEquals-002 fail it",
    ),
    (
        "node-expression-target-node",
        &[(
            "NodeKindConstraintComponent",
            "<http://www.w3.org/ns/shacl#targetNode>",
            "",
        )],
        2,
        "SHACL 1.2 Core, \"Node targets\": \"Each value of sh:targetNode in a shape is a \
         well-formed node expression.\" A blank node that is the subject of no triple is the \
         empty node expression, and one carrying sh:select is a SPARQL node expression whose \
         output nodes are the targets (targetNode-select-001); shacl-shacl.ttl still requires \
         an IRI or a literal. 1 until a structured sh:targetNode became evaluated and \
         targetNode-select-001 stopped being refused at load",
    ),
    (
        "sequence-path-with-other-values",
        &[(
            "XoneConstraintComponent",
            "",
            "<http://www.w3.org/ns/shacl-shacl#ShapeShape>",
        )],
        1,
        "SHACL 1.2 Core §2.3.1: \"An inverse path is a blank node that is the subject of \
         exactly one triple in G.\" A sequence-path node that also carries sh:inversePath is a \
         sequence path, and the sh:inversePath value is no path of it (the W3C suite's \
         core/path/path-strange-002 validates exactly that path as the sequence). \
         shacl-shacl.ttl's path walk follows sh:inversePath from every path node, judges the \
         one-member list there as a path, and so fails the property shape's sh:node \
         shsh:PathShape — which surfaces as the sh:xone of shsh:ShapeShape, the only result; \
         the exact count keeps this pattern from absorbing any other input",
    ),
];

// ── Inputs ────────────────────────────────────────────────────────────────────

/// One shapes graph under test.
struct Input {
    id: String,
    dataset: Arc<RdfDataset>,
    prefixes: Vec<(String, String)>,
    box_vocab: Option<BoxRoleVocab>,
    graph_iri: Option<String>,
}

fn parse(path: &Path) -> (Arc<RdfDataset>, Vec<(String, String)>) {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let document = parse_turtle_document(&text, Some(&file_iri(path)))
        .unwrap_or_else(|e| panic!("{} does not parse: {e:?}", path.display()));
    (document.dataset, document.prefixes)
}

/// Every shapes graph of the three corpora, once per distinct file.
fn inputs() -> Vec<Input> {
    let mut out: Vec<Input> = Vec::new();
    for case in first_party_cases() {
        let (dataset, prefixes) = parse(&case.shapes_path);
        out.push(Input {
            id: format!("corpus/{}", case.name),
            dataset,
            prefixes,
            box_vocab: Some(first_party_box_role_vocab()),
            graph_iri: None,
        });
    }
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    for case in w3c_cases() {
        if seen.insert(case.shapes_path.clone()) {
            let (dataset, prefixes) = parse(&case.shapes_path);
            out.push(Input {
                id: format!("w3c/{}", case.id),
                dataset,
                prefixes,
                box_vocab: None,
                graph_iri: case.shapes_graph_iri,
            });
        }
    }
    for case in shacl12_cases() {
        if let Body::Validate(validate) = &case.body
            && seen.insert(validate.shapes_path.clone())
        {
            let (dataset, prefixes) = parse(&validate.shapes_path);
            out.push(Input {
                id: format!("w3c12/{}", case.id),
                dataset,
                prefixes,
                box_vocab: None,
                graph_iri: validate.shapes_graph_iri.clone(),
            });
        }
    }
    out
}

// ── The two judges ────────────────────────────────────────────────────────────

/// PurRDF's parser: `Err` is a refusal, with its message.
fn purrdf_refusal(input: &Input) -> Option<String> {
    from_dataset_with_config_and_graph(
        &input.dataset,
        &input.prefixes,
        input.box_vocab.clone(),
        input.graph_iri.clone(),
    )
    .err()
}

/// One `sh:Violation` result of `shacl-shacl.ttl`: `(component, result path, source
/// shape)`, the latter two empty when absent or a blank node.
type Violation = (String, String, String);

/// `shacl-shacl.ttl`'s verdict on `dataset` as data: every `sh:Violation` result,
/// sorted.
fn shacl_shacl_violations(oracle: &Shapes, dataset: &RdfDataset) -> Vec<Violation> {
    let report = validate_dataset_with_shapes_graph(dataset, oracle, None)
        .unwrap_or_else(|e| panic!("shacl-shacl.ttl failed to validate a shapes graph: {e}"));
    let named = |term: &purrdf_shapes::term::Term| match term {
        purrdf_shapes::term::Term::NamedNode(_) => term.to_string(),
        _ => String::new(),
    };
    let mut out: Vec<Violation> = report
        .results
        .iter()
        .filter(|result| result.severity == Severity::Violation)
        .map(|result| {
            (
                result.source_constraint_component.as_str().to_owned(),
                result.result_path.as_ref().map_or_else(String::new, named),
                named(&result.source_shape),
            )
        })
        .collect();
    out.sort();
    out
}

// ── Mutants ───────────────────────────────────────────────────────────────────

/// One `N-Triples` statement of a shapes graph, split into its three terms.
struct Statement {
    subject: String,
    predicate: String,
    object: String,
}

fn statements(dataset: &RdfDataset) -> Vec<Statement> {
    let bytes = serialize_dataset(dataset, "application/n-quads", SerializeGraph::DefaultGraph)
        .expect("a shapes graph serializes as N-Triples");
    let text = String::from_utf8(bytes).expect("N-Triples is UTF-8");
    let mut out: Vec<Statement> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let line = line.trim_end();
            let body = line
                .strip_suffix('.')
                .expect("an N-Triples line ends in '.'")
                .trim_end();
            let (subject, rest) = body.split_once(' ').expect("a subject");
            let (predicate, object) = rest.split_once(' ').expect("a predicate");
            Statement {
                subject: subject.to_owned(),
                predicate: predicate.to_owned(),
                object: object.to_owned(),
            }
        })
        .collect();
    out.sort_by(|a, b| {
        (&a.subject, &a.predicate, &a.object).cmp(&(&b.subject, &b.predicate, &b.object))
    });
    out
}

fn sh(local: &str) -> String {
    format!("<{SH}{local}>")
}

/// A mutation: its name, which predicates it applies to, and how it rewrites the
/// first matching statement (the replacement object, plus any statements to add).
struct Mutation {
    name: &'static str,
    predicates: &'static [&'static str],
    rewrite: fn(&Statement) -> (String, Vec<String>),
}

const MUTATIONS: &[Mutation] = &[
    Mutation {
        name: "literal-where-an-IRI-is-required",
        predicates: &[
            "class",
            "targetClass",
            "targetSubjectsOf",
            "targetObjectsOf",
            "equals",
            "disjoint",
            "lessThan",
            "lessThanOrEquals",
            "severity",
            "rootClass",
        ],
        rewrite: |_| ("\"not an IRI\"".to_owned(), Vec::new()),
    },
    Mutation {
        name: "non-integer-count",
        predicates: &[
            "minCount",
            "maxCount",
            "minLength",
            "maxLength",
            "qualifiedMinCount",
            "qualifiedMaxCount",
        ],
        rewrite: |_| ("\"many\"".to_owned(), Vec::new()),
    },
    Mutation {
        name: "list-where-a-single-value-is-required",
        predicates: &[
            "minCount",
            "maxCount",
            "minLength",
            "maxLength",
            "pattern",
            "flags",
            "datatype",
            "singleLine",
        ],
        rewrite: |_| {
            (
                "_:purrdfMutantList".to_owned(),
                vec![
                    format!("_:purrdfMutantList <{RDF}first> \"1\"^^<{XSD_INTEGER}> ."),
                    format!("_:purrdfMutantList <{RDF}rest> <{RDF}nil> ."),
                ],
            )
        },
    },
    Mutation {
        name: "non-boolean-flag",
        predicates: &[
            "closed",
            "uniqueLang",
            "deactivated",
            "qualifiedValueShapesDisjoint",
            "reificationRequired",
            "uniqueMembers",
            "singleLine",
        ],
        rewrite: |_| ("\"yes\"".to_owned(), Vec::new()),
    },
];

/// The misspelled-parameter mutant: a property shape gains `sh:minCont 1`.
const UNKNOWN_TERM: &str = "unknown-sh-predicate";

/// Every mutant of `base`, as `(mutation name, N-Triples text)`.
fn mutants(base: &RdfDataset) -> Vec<(&'static str, String)> {
    let all = statements(base);
    let render = |replace: Option<(usize, &str)>, extra: &[String]| -> String {
        let mut text = String::new();
        for (index, statement) in all.iter().enumerate() {
            let object = match replace {
                Some((at, object)) if at == index => object,
                _ => statement.object.as_str(),
            };
            writeln!(
                text,
                "{} {} {} .",
                statement.subject, statement.predicate, object
            )
            .expect("writing to a String cannot fail");
        }
        for line in extra {
            text.push_str(line);
            text.push('\n');
        }
        text
    };
    let mut out = Vec::new();
    for mutation in MUTATIONS {
        let wanted: Vec<String> = mutation.predicates.iter().map(|local| sh(local)).collect();
        if let Some((index, statement)) = all
            .iter()
            .enumerate()
            .find(|(_, statement)| wanted.contains(&statement.predicate))
        {
            let (object, extra) = (mutation.rewrite)(statement);
            out.push((mutation.name, render(Some((index, &object)), &extra)));
        }
    }
    if let Some(statement) = all
        .iter()
        .find(|statement| statement.predicate == sh("path"))
    {
        let extra = vec![format!(
            "{} {} \"1\"^^<{XSD_INTEGER}> .",
            statement.subject,
            sh("minCont")
        )];
        out.push((UNKNOWN_TERM, render(None, &extra)));
    }
    out
}

// ── The oracle ────────────────────────────────────────────────────────────────

/// How one input came out.
#[derive(Debug)]
enum Outcome {
    /// Both accept, or both refuse.
    Agree,
    /// PurRDF refuses what shacl-shacl accepts, under the named stricter rule.
    Stricter(&'static str),
    /// shacl-shacl flags what PurRDF accepts, every violation under a named spec
    /// entry.
    BehindTheSpec(BTreeSet<&'static str>),
    /// A disagreement no ledger entry explains.
    Unexplained(String),
}

fn judge(id: &str, refusal: Option<&str>, violations: &[Violation]) -> Outcome {
    match (refusal, violations.is_empty()) {
        (Some(_), false) | (None, true) => Outcome::Agree,
        (Some(message), true) => STRICTER_THAN_SHACL_SHACL
            .iter()
            .find(|(_, needle, _, _)| message.contains(needle))
            .map_or_else(
                || {
                    Outcome::Unexplained(format!(
                        "[{id}] PurRDF refuses and shacl-shacl.ttl accepts: {message}"
                    ))
                },
                |(name, ..)| Outcome::Stricter(name),
            ),
        (None, false) => {
            let mut names: BTreeSet<&'static str> = BTreeSet::new();
            for (component, path, shape) in violations {
                let entry = SHACL_SHACL_BEHIND_THE_SPEC
                    .iter()
                    .find(|(_, covered, _, _)| {
                        covered.iter().any(|(c, p, s)| {
                            component.ends_with(&format!("#{c}")) && path == p && shape == s
                        })
                    });
                match entry {
                    Some((name, ..)) => {
                        names.insert(name);
                    }
                    None => {
                        return Outcome::Unexplained(format!(
                            "[{id}] shacl-shacl.ttl flags and PurRDF accepts (an \
                             under-refusal): {violations:?}"
                        ));
                    }
                }
            }
            Outcome::BehindTheSpec(names)
        }
    }
}

/// The exact number of shapes graphs the three corpora contribute: 73 first-party
/// cases, 129 W3C SHACL 1.0 cases and 174 W3C SHACL 1.2 `sht:Validate` entries, each
/// with its own shapes file.
const BASE_INPUTS: usize = 376;

/// The exact number of mutants generated from the bases both sides accept (one per
/// mutation kind that finds a statement to rewrite): 192 literal-for-IRI, 101
/// non-integer counts, 157 lists-for-single-values, 35 non-boolean flags and 235
/// misspelled predicates.
///
/// Moved from 683 to 690 when `sh:singleLine`, `sh:rootClass` and `sh:someValue`
/// became evaluated: `singleLine-001`, `rootClass-001` and `someValue-001` now
/// load, so each is a base both sides accept and is mutated. All three gain a
/// misspelled predicate (+3). The literal-for-IRI kind, which now also rewrites
/// `sh:rootClass`, rewrites `rootClass-001`'s root class and the `sh:class` inside
/// `someValue-001`'s `sh:someValue` shape (+2). The list-for-single-value kind
/// rewrites `singleLine-001`'s `sh:datatype` (+1), and the non-boolean-flag kind,
/// which now also rewrites `sh:singleLine`, rewrites its `sh:singleLine` (+1).
///
/// Moved from 690 to 692 when `sh:subsetOf` became evaluated: `subsetOf-001` and
/// `subsetOf-002` now load, so each is a base both sides accept, and each gains a
/// misspelled predicate (+2). No other kind finds a statement to rewrite in
/// either. The path-valued `equals-002`, `disjoint-002`, `lessThan-003` and
/// `lessThanOrEquals-002` also load now, but `shacl-shacl.ttl` still flags each
/// (see `path-valued-property-pair`), so none is a base both sides accept and
/// none is mutated.
///
/// Moved from 692 to 697 when `sh:uniqueValuesFor` became evaluated:
/// `uniqueValuesFor-001` to `-005` now load, so each is a base both sides accept.
/// The literal-for-IRI kind rewrites each one's `sh:targetClass` or
/// `sh:targetSubjectsOf`, the first matching statement in canonical order (+5).
/// None has a property shape, so none gains a misspelled predicate.
///
/// Moved from 697 to 708 when per-constraint reifier annotations and the
/// `sh:Debug` / `sh:Trace` severities became evaluated: `deactivated-003`,
/// `severity-003`, `severity-004`, `severity-005` and `message-002` now load, so
/// each is a base both sides accept. `deactivated-003`, whose property shape
/// carries `sh:minCount`, gains a non-integer count, a list-for-single-value, a
/// non-boolean flag (its reifier `sh:deactivated`, refused under
/// `reifier-annotation-value`) and a misspelled predicate (+4); `severity-003`,
/// `-004` and `-005` each gain a literal-for-IRI (for `severity-003` its reifier
/// `sh:severity`, refused under `reifier-annotation-value`) and a
/// list-for-single-value (+6); `message-002` gains a list-for-single-value (+1).
///
/// Moved from 708 to 712 when implicit class targets, `sh:ShapeClass` and
/// `sh:targetWhere` became evaluated: `targetClassImplicit-002` and
/// `targetWhere-001` now load, so each is a base both sides accept.
/// `targetWhere-001` gains a literal-for-IRI (its first `sh:class`), a non-integer
/// count and a list-for-single-value (its first `sh:minCount`) and a misspelled
/// predicate (+4); `targetClassImplicit-002`, whose one shape is a `sh:ShapeClass`
/// carrying only `sh:in`, has no statement any mutation rewrites.
/// `targetNode-select-001` loads too, but `shacl-shacl.ttl` flags its structured
/// `sh:targetNode` (see `node-expression-target-node`), so it is not a base both
/// sides accept and is not mutated.
///
/// Moved from 712 to 717 when `sh:values` and `sh:defaultValue` became
/// evaluated: `property-select-001` and `property-sparqlExpr-001` now load, so
/// each is a base both sides accept. The list-for-single-value kind rewrites each
/// one's `sh:datatype` (+2), the literal-for-IRI kind `property-select-001`'s
/// `sh:targetClass` (+1), and each property shape gains a misspelled predicate
/// (+2); `property-sparqlExpr-001` targets by `sh:targetNode`, which no kind
/// rewrites.
///
/// Moved from 717 to 719 when `shnex:instancesOf` took a node-expression
/// argument: `sparql/functions/instanceCount-example` now loads, so it is a base
/// both sides accept. The literal-for-IRI kind rewrites its SPARQL constraint's
/// `sh:severity` (+1, refused under `sparql-constraint-severity`), and its shape
/// gains a misspelled predicate (+1, refused under `unknown-term`).
///
/// Moved from 719 to 720 when the first-party corpus gained
/// `73-expr-if-list-true`, a base both sides accept: the literal-for-IRI kind
/// rewrites its `sh:targetClass` (+1). Its one shape is a node shape with no
/// property shape, so it gains no misspelled predicate.
///
/// Moved from 720 to 724 when the `sh:RulesEntailment` regime became supported:
/// `inference-rules/rules-entailment-validation` now loads, so it is a base both
/// sides accept. The literal-for-IRI kind rewrites its `sh:targetClass` (+1), the
/// non-integer-count and list-for-single-value kinds its property shape's
/// `sh:minCount` (+2), and the property shape gains a misspelled predicate (+1,
/// refused under `unknown-term`).
const MUTANT_INPUTS: usize = 724;

#[test]
fn purrdf_refuses_exactly_what_shacl_shacl_flags() {
    let oracle_document = parse_turtle_document(SHACL_SHACL, None).expect("shacl-shacl.ttl parses");
    let oracle_dataset = oracle_document.dataset;
    let oracle =
        from_dataset_with_config_and_graph(&oracle_dataset, &oracle_document.prefixes, None, None)
            .expect("shacl-shacl.ttl loads as a shapes graph");

    let mut unexplained: Vec<String> = Vec::new();
    let mut stricter: BTreeMap<&str, usize> = BTreeMap::new();
    let mut behind: BTreeMap<&str, usize> = BTreeMap::new();
    let mut tally = |outcome: Outcome| match outcome {
        Outcome::Agree => {}
        Outcome::Stricter(name) => *stricter.entry(name).or_insert(0) += 1,
        Outcome::BehindTheSpec(names) => {
            for name in names {
                *behind.entry(name).or_insert(0) += 1;
            }
        }
        Outcome::Unexplained(why) => unexplained.push(why),
    };

    let bases = inputs();
    let mut mutant_count = 0usize;
    // Every mutation makes the graph ill-formed, so a mutant PurRDF ACCEPTS is an
    // under-refusal whatever shacl-shacl.ttl says about it.
    let mut accepted_mutants: Vec<String> = Vec::new();
    let mut mutants_by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for input in &bases {
        let refusal = purrdf_refusal(input);
        let violations = shacl_shacl_violations(&oracle, &input.dataset);
        let clean = refusal.is_none() && violations.is_empty();
        tally(judge(&input.id, refusal.as_deref(), &violations));
        if !clean {
            continue;
        }
        for (kind, text) in mutants(&input.dataset) {
            mutant_count += 1;
            *mutants_by_kind.entry(kind).or_insert(0) += 1;
            let dataset = purrdf::parse_dataset(text.as_bytes(), "application/n-quads", None)
                .unwrap_or_else(|e| panic!("[{}] mutant {kind} does not parse: {e}", input.id));
            let mutant = Input {
                id: format!("{} + {kind}", input.id),
                dataset,
                prefixes: input.prefixes.clone(),
                box_vocab: input.box_vocab.clone(),
                graph_iri: input.graph_iri.clone(),
            };
            let refusal = purrdf_refusal(&mutant);
            if refusal.is_none() {
                accepted_mutants.push(mutant.id.clone());
            }
            let violations = shacl_shacl_violations(&oracle, &mutant.dataset);
            tally(judge(&mutant.id, refusal.as_deref(), &violations));
        }
    }

    println!(
        "SHACL-SHACL DIFFERENTIAL: {} bases, {mutant_count} mutants {mutants_by_kind:?}; \
         stricter {stricter:?}; behind the spec {behind:?}",
        bases.len()
    );
    assert!(
        accepted_mutants.is_empty(),
        "PurRDF accepts {} ill-formed mutant(s): {accepted_mutants:?}",
        accepted_mutants.len()
    );
    assert!(
        unexplained.is_empty(),
        "{} disagreement(s) no ledger explains:\n{}",
        unexplained.len(),
        unexplained.join("\n")
    );
    for (name, _, expected, _) in STRICTER_THAN_SHACL_SHACL {
        assert_eq!(
            stricter.get(name).copied().unwrap_or(0),
            *expected,
            "STRICTER_THAN_SHACL_SHACL[{name}] covers a different number of inputs"
        );
    }
    for (name, _, expected, _) in SHACL_SHACL_BEHIND_THE_SPEC {
        assert_eq!(
            behind.get(name).copied().unwrap_or(0),
            *expected,
            "SHACL_SHACL_BEHIND_THE_SPEC[{name}] covers a different number of inputs"
        );
    }
    assert_eq!(bases.len(), BASE_INPUTS, "base shapes-graph count");
    assert_eq!(mutant_count, MUTANT_INPUTS, "mutant count");
    let expected_by_kind: BTreeMap<&str, usize> = [
        ("literal-where-an-IRI-is-required", 193),
        ("non-integer-count", 102),
        ("list-where-a-single-value-is-required", 158),
        ("non-boolean-flag", 35),
        (UNKNOWN_TERM, 236),
    ]
    .into_iter()
    .collect();
    assert_eq!(mutants_by_kind, expected_by_kind, "mutants per kind");
}
