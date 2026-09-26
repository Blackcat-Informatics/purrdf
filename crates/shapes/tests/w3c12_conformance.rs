// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! W3C SHACL 1.2 conformance harness over the vendored `vectors/shacl12/tests`
//! suite (w3c/data-shapes `shacl12-test-suite`), run through the LIBRARY API.
//!
//! Discovery is [`shacl_corpora::shacl12`]: every manifest root, every test type,
//! with the reachability guards that keep "discovered" and "on disk" the same
//! corpus. This file GRADES each discovered entry by its type:
//!
//! * **`sht:Validate`** — [`shacl_corpora::report_grading::run_validate_case`],
//!   the grader the SHACL 1.0 harness uses, so both suites mean the same thing by
//!   "the report agrees": `sh:conforms` plus the multiset of
//!   `(focusNode, resultPath, value, sourceConstraintComponent, severity,
//!   sourceShape)` tuples, blank nodes normalized, every `sh:resultMessage` the expected report
//!   mentions, every `sh:detail` it states (recursively), and — where the expected report states `sh:conformanceDisallows`
//!   — validation under exactly that set, echoed back in the report. `sht:Failure` expects an error at load or
//!   validation. An approved (manifest-listed) entry is graded against its
//!   approved expectation and nothing else; [`UNLISTED_FILE_DELTAS`] amends only
//!   entries of vendored files no upstream manifest includes.
//! * **`sht:EvalNodeExpr`** — the `sht:nodeExpr` node is parsed by the shapes
//!   parser itself ([`purrdf_shapes::shapes::from_dataset_with_node_expressions`],
//!   so custom functions and shape references bind exactly as they do in a
//!   shapes graph), and evaluated with `sht:focusNode` as the focus node and every
//!   `sht:scope-NAME` bound through the public [`Scope`]/[`Binding`] API. The
//!   output must equal the `mf:result` list TERM FOR TERM (RDF 1.2 term
//!   equality: lexical form, datatype, language and direction), in order unless
//!   the entry sets `sht:ignoreOrder true`, in which case as a multiset. The only
//!   departure from exact equality is [`NON_CANONICAL_EXPECTATIONS`].
//! * **`sht:Infer`** — the shapes graph's rules run through
//!   [`purrdf_shapes::apply_rules`] over the data graph, and the inferred graph
//!   must equal `data ⊎ expected` under RDF isomorphism (RDFC-1.0 canonical
//!   N-Quads, as the SHACL Rules harness compares). The expectation is the inline
//!   triple list, `rdf:nil`, or a result file; `sht:Failure` expects an error.
//! * **`srlt:*`** — the seven SPARQL 1.2 RL test types, routed to [`run_srl`].
//!
//! ## An entry with no `sht:focusNode`
//!
//! The suite's node-expression README makes `sht:focusNode` optional and says
//! nothing of what the focus node then is. The library's evaluator always takes
//! one, so such an entry is evaluated from [`ABSENT_FOCUS`], a blank node the
//! harness proves occurs nowhere in the test graph: it has no path values, is an
//! instance of nothing and conforms to nothing the graph could say about it.
//!
//! ## What is counted, and under which label
//!
//! [`w3c_shacl12_conformance`] grades the APPROVED suite — every entry an
//! upstream manifest lists — and only those entries count as its passes. The
//! entries of vendored files no upstream manifest includes are graded by
//! [`w3c_shacl12_unlisted_vendored_files`] and reported under their own label,
//! never added to the pass count.
//!
//! ## Ledger
//!
//! [`XFAIL`] names every entry the engine fails today, with the reason. The
//! harness asserts that every ledgered entry still fails (an `XPASS` is an error:
//! remove the entry), that every other entry passes, and the exact discovered
//! total. Run with `--nocapture` for the per-section and per-type scoreboard:
//! `cargo test -p purrdf-shapes --test w3c12_conformance -- --nocapture`

mod shacl_corpora;

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use purrdf::{RdfDataset, RdfDatasetBuilder, RdfQuad, canonicalize};
use purrdf_shapes::data::{GraphFilter, ShaclData, native_quads};
use purrdf_shapes::expression::{
    Binding, NodeExpr, RecursionGuard, Scope, eval_node_expr_in_scope,
};
use purrdf_shapes::srl::{self, SrlError};
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_shapes::{apply_rules, engine, shapes, sparql, text_ingest};

use shacl_corpora::report_grading::{
    grade, grade_against, grade_refused_import, no_panic, produce,
};
use shacl_corpora::shacl12::{
    Body, Case12, InferCase, InferExpected, NodeExprCase, SrlCase, SrlKind, W3C12_TOTAL_CASES,
    shacl12_cases,
};
use shacl_corpora::{Expected, Multiset, Tuple, W3cCase, file_iri, parse_turtle_file};

// ── Xfail ledger ──────────────────────────────────────────────────────────────

/// `rdf:reifies`.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Entries the engine currently fails, with the reason: `(test id, reason)`,
/// where the id is the entry's IRI relative to `vectors/shacl12/tests` (see
/// `shacl12::entry_id` for the SPARQL 1.2 RL evaluation entries). Grouped by
/// feature; each reason names what the engine does today, not a plan.
///
/// A ledgered entry MUST fail; when engine work fixes it the harness errors with
/// `XPASS` and the entry must be removed.
const XFAIL: &[(&str, &str)] = &[];

// ── Upstream errata: non-canonical expectations ──────────────────────────────

/// UPSTREAM ERRATA: node-expression entries whose W3C expected literal is NOT in
/// the canonical lexical form XSD 1.1 Part 2 assigns its value: `(test id,
/// expected lexical, XSD 1.1 canonical lexical, canonical-mapping clause)`.
///
/// Each expects an integer-valued `xsd:decimal` spelled the XSD 1.0 way (`"4.0"`,
/// `"00"`); PurRDF emits the XSD 1.1 canonical form (`"4"`, `"0"`), as the
/// approved W3C SPARQL suite itself expects of CEIL, FLOOR, ROUND and SECONDS
/// (`functions#ceil01`, `floor01`, `round01`, `seconds` expect `"3"`, `"2"`,
/// `"1"`, `"0"`). These entries are therefore NOT passes of the approved suite:
/// the harness reports them under their own `upstream-errata` label, pinned by
/// [`NON_CANONICAL_EXPECTATIONS_COUNT`], and grades each one exactly as follows.
///
/// RDF 1.2 literal equality compares lexical forms, and PurRDF emits canonical
/// forms, so an expectation spelled non-canonically can never be met by a
/// canonical engine — and it is not relaxed by comparing VALUES instead, which
/// would equally accept a genuinely non-canonical engine output. Each entry
/// instead swaps the one expected lexical form for its canonical form and grades
/// the output against THAT, exactly. The table is checked in both directions:
/// [`non_canonical_expectations_are_really_non_canonical`] proves each expected
/// form is non-canonical and that the stated canonical form is what the XSD 1.1
/// canonical mapping produces, and the harness proves the engine's output equals
/// the canonical form term for term.
/// The clause every current entry cites. XSD 1.1 Part 2 §3.3.3.1 states that
/// "for integers, the decimal point and fractional part are prohibited" in the
/// canonical representation and that the mapping "is given formally in
/// decimalCanonicalMap", which §E.1 defines: "If d is an integer, then return
/// noDecimalPtCanonicalMap(d)". SPARQL's CEIL, FLOOR, ROUND, numeric division,
/// SECONDS and SUM over decimals all yield `xsd:decimal`.
const DECIMAL_CANONICAL_MAP: &str = "XSD 1.1 Part 2 §3.3.3.1 + §E.1 decimalCanonicalMap: \
     an integer-valued xsd:decimal maps through noDecimalPtCanonicalMap (no decimal point)";

const NON_CANONICAL_EXPECTATIONS: &[(&str, &str, &str, &str)] = &[
    (
        "node-expr/shnex-sparql/ceil-example",
        "4.0",
        "4",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/floor-example",
        "3.0",
        "3",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/round-example",
        "4.0",
        "4",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/divide-example",
        "42.0",
        "42",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex-sparql/seconds-example",
        "00",
        "0",
        DECIMAL_CANONICAL_MAP,
    ),
    (
        "node-expr/shnex/sum-totalRevenue",
        "42.0",
        "42",
        DECIMAL_CANONICAL_MAP,
    ),
];

/// [`NON_CANONICAL_EXPECTATIONS`] pinned by count, so an entry cannot be added or
/// dropped without this number moving with it.
const NON_CANONICAL_EXPECTATIONS_COUNT: usize = 6;

/// Replace each expected literal an entry of [`NON_CANONICAL_EXPECTATIONS`]
/// names with its canonical form. An entry that matches no expected literal of
/// its test is a stale entry and an error.
fn canonical_expectation(id: &str, expected: &[Term]) -> Result<Vec<Term>, String> {
    let Some((_, lexical, canonical, _)) = NON_CANONICAL_EXPECTATIONS
        .iter()
        .find(|(entry, ..)| *entry == id)
    else {
        return Ok(expected.to_vec());
    };
    let mut replaced = 0usize;
    let out = expected
        .iter()
        .map(|term| match term {
            Term::Literal(l) if l.value() == *lexical && l.language().is_none() => {
                replaced += 1;
                Term::Literal(Literal::new_typed_literal(*canonical, l.datatype()))
            }
            other => other.clone(),
        })
        .collect();
    if replaced == 0 {
        return Err(format!(
            "NON_CANONICAL_EXPECTATIONS names {id} with expected lexical {lexical:?}, which \
             is not among its expected results — stale entry"
        ));
    }
    Ok(out)
}

// ── Unlisted vendored files ───────────────────────────────────────────────────

/// One compared result tuple, spelled as the comparison spells it (see
/// `shacl_corpora::norm`): an IRI in angle brackets, a literal or triple term in
/// N-Triples form, any blank node as `_:`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResultTuple {
    focus: &'static str,
    path: Option<&'static str>,
    value: Option<&'static str>,
    component: &'static str,
    severity: &'static str,
    source_shape: &'static str,
}

impl ResultTuple {
    /// This tuple as a multiset key.
    fn key(self) -> Tuple {
        (
            self.focus.to_owned(),
            self.path.map(ToOwned::to_owned),
            self.value.map(ToOwned::to_owned),
            self.component.to_owned(),
            self.severity.to_owned(),
            self.source_shape.to_owned(),
        )
    }
}

/// One amendment of one approved expected result: the approved report's
/// `approved` result is graded as `graded` instead. Every field the two do not
/// share IS the delta; every field they share is held.
#[derive(Clone, Copy, Debug)]
struct Amendment {
    approved: ResultTuple,
    graded: ResultTuple,
}

const XONE_PROPERTY_SHAPE: &str =
    "<http://www.w3.org/ns/shacl-shacl#xoneSubjectsShapeXonePropertyShape>";

/// Deltas for the entries of vendored files that NO upstream manifest includes
/// (`shacl12::W3C12_UNINCLUDED_MANIFESTS`), where the file's own expectation
/// contradicts normative SHACL 1.2 Core text: `(test id, the normative sentence
/// quoted with its section, the exact delta)`.
///
/// Such an entry is not part of the approved suite — no manifest lists it — so it
/// is never counted among the suite's passes: the harness grades and reports it
/// under its own label (see [`w3c_shacl12_unlisted_vendored_files`]). A LISTED
/// entry can never be named here; [`graded_expectation`] refuses it, so no
/// approved test is ever graded against anything but its approved expectation.
///
/// An entry is not an expected failure. The harness grades the engine's report
/// against the file's expected report WITH the delta applied, and against
/// nothing else: a report that lacks the delta, carries a different one, or
/// differs anywhere else fails exactly as an unamended comparison would. Each
/// delta must apply to a result the file's report really contains, or the
/// entry is stale and the harness says so. [`unlisted_file_deltas_are_exact`]
/// proves both directions on the engine's real report, and pins the table by
/// count.
///
/// * `core/node/xone-003` (`core/node/xone-003.ttl`, which
///   `core/node/manifest.ttl` does not include) — the result is produced by
///   `shsh:xoneSubjectsShapeXonePropertyShape`, a property shape whose `sh:path`
///   is `sh:xone`, through `sh:minListLength`, whose definition states no
///   exception to §6.7.2.2. The file's report omits `sh:resultPath`; the quoted
///   sentence makes it the shape's `sh:path`, and the engine keeps it.
const UNLISTED_FILE_DELTAS: &[(&str, &str, &[Amendment])] = &[(
    "core/node/xone-003",
    "SHACL 1.2 Core §6.7.2.2: \"For results produced by a property shape, this SHACL \
         property path is equivalent to the value of sh:path of the shape, unless stated \
         otherwise.\"",
    &[Amendment {
        approved: ResultTuple {
            focus: "<http://example.com/ns#TestXoneUnsatisfiableShape>",
            path: None,
            value: Some("<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>"),
            component: "<http://www.w3.org/ns/shacl#MinListLengthConstraintComponent>",
            severity: "<http://www.w3.org/ns/shacl#Warning>",
            source_shape: XONE_PROPERTY_SHAPE,
        },
        graded: ResultTuple {
            focus: "<http://example.com/ns#TestXoneUnsatisfiableShape>",
            path: Some("<http://www.w3.org/ns/shacl#xone>"),
            value: Some("<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>"),
            component: "<http://www.w3.org/ns/shacl#MinListLengthConstraintComponent>",
            severity: "<http://www.w3.org/ns/shacl#Warning>",
            source_shape: XONE_PROPERTY_SHAPE,
        },
    }],
)];

/// [`UNLISTED_FILE_DELTAS`] pinned by count, so an entry cannot be added or
/// dropped without this number moving with it.
const UNLISTED_FILE_DELTAS_COUNT: usize = 1;

/// The approved expected results of `id` with `deltas` applied. A delta whose
/// approved result the approved report does not contain is a stale entry and an
/// error, so an entry can never amend a result into existence.
fn amend(id: &str, results: &Multiset, deltas: &[Amendment]) -> Result<Multiset, String> {
    let mut amended = results.clone();
    for delta in deltas {
        let approved = delta.approved.key();
        match amended.get_mut(&approved) {
            Some(count) if *count > 1 => *count -= 1,
            Some(_) => {
                amended.remove(&approved);
            }
            None => {
                return Err(format!(
                    "UNLISTED_FILE_DELTAS amends {id} at {approved:?}, which is not among its \
                     approved expected results — stale entry"
                ));
            }
        }
        *amended.entry(delta.graded.key()).or_insert(0) += 1;
    }
    Ok(amended)
}

/// The expectation a validation case is graded against: the file's own, or — for
/// an unlisted entry only — the file's own amended by its [`UNLISTED_FILE_DELTAS`]
/// entry. A listed (approved-suite) entry named in that table is an error.
fn graded_expectation(id: &str, listed: bool, tc: &W3cCase) -> Result<Expected, String> {
    let entry = UNLISTED_FILE_DELTAS.iter().find(|(entry, ..)| *entry == id);
    if listed && entry.is_some() {
        return Err(format!(
            "UNLISTED_FILE_DELTAS names {id}, which an upstream manifest lists — an approved \
             test is graded against its approved expectation only"
        ));
    }
    let Some((_, _, deltas)) = entry else {
        return Ok(match &tc.expected {
            Expected::Failure => Expected::Failure,
            Expected::Report { conforms, results } => Expected::Report {
                conforms: *conforms,
                results: results.clone(),
            },
        });
    };
    let Expected::Report { conforms, results } = &tc.expected else {
        return Err(format!(
            "UNLISTED_FILE_DELTAS names {id}, whose approved result is sht:Failure — a result \
             delta cannot amend it"
        ));
    };
    Ok(Expected::Report {
        conforms: *conforms,
        results: amend(id, results, deltas)?,
    })
}

/// Grade one `sht:Validate` entry against its graded expectation.
fn run_validate(id: &str, listed: bool, tc: &W3cCase) -> Result<(), String> {
    grade_against(tc, &graded_expectation(id, listed, tc)?)
}

// ── Term-level comparison ─────────────────────────────────────────────────────

/// Compare produced node-expression output against the expected list: exact RDF
/// 1.2 term equality, in order, or as a multiset under `ignore_order`.
fn compare_outputs(produced: &[Term], expected: &[Term], ignore_order: bool) -> Result<(), String> {
    let agree = if ignore_order {
        let mut p: Vec<&Term> = produced.iter().collect();
        let mut e: Vec<&Term> = expected.iter().collect();
        p.sort_by_key(ToString::to_string);
        e.sort_by_key(ToString::to_string);
        p == e
    } else {
        produced == expected
    };
    if agree {
        return Ok(());
    }
    let render = |terms: &[Term]| {
        terms
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    };
    Err(format!(
        "node-expression output mismatch{}:\n  produced ( {} )\n  expected ( {} )",
        if ignore_order { " (order ignored)" } else { "" },
        render(produced),
        render(expected)
    ))
}

// ── sht:EvalNodeExpr ──────────────────────────────────────────────────────────

/// The focus node of an entry that gives no `sht:focusNode` (see module docs).
const ABSENT_FOCUS: &str = "shacl12-harness-absent-focus-node";

/// Evaluate `expr` with `bindings` pushed as scope frames on top of `scope`.
fn eval_in_scope(
    data: &ShaclData,
    focus: &Term,
    expr: &NodeExpr,
    guard: &mut RecursionGuard,
    bindings: &[(String, Term)],
    scope: Scope<'_>,
) -> Result<Vec<Term>, String> {
    match bindings.split_first() {
        None => eval_node_expr_in_scope(data, focus, expr, guard, scope),
        Some(((name, value), rest)) => {
            let frame = Binding::new(name, value, scope);
            eval_in_scope(data, focus, expr, guard, rest, Scope::bound(&frame))
        }
    }
}

/// Parse and evaluate an entry's node expression through the library.
fn eval_node_expr_case(tc: &NodeExprCase) -> Result<Vec<Term>, String> {
    let text = fs::read_to_string(&tc.file)
        .map_err(|e| format!("cannot read {}: {e}", tc.file.display()))?;
    let doc_prefixes = text_ingest::parse_turtle_document(&text, Some(&file_iri(&tc.file)))
        .map_err(|errors| format!("node-expression graph parse error: {}", errors.join("; ")))?
        .prefixes;
    let (shapes, mut exprs) = shapes::from_dataset_with_node_expressions(
        &tc.dataset,
        &doc_prefixes,
        None,
        std::slice::from_ref(&tc.expr),
        &shacl_corpora::w3c_case_imports(&tc.dataset),
    )
    .map_err(|e| format!("shapes/node-expression parse error: {e}"))?;
    let expr = exprs
        .pop()
        .ok_or("the parser returned no expression for the one root it was given")?;

    let projected = engine::project_dataset(tc.dataset.as_ref())?;
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    let focus = match &tc.focus {
        Some(focus) => focus.clone(),
        None => {
            let absent = Term::blank(ABSENT_FOCUS);
            let mentioned = !native_quads(
                tc.dataset.as_ref(),
                Some(&absent),
                None,
                None,
                GraphFilter::AnyGraph,
            )
            .is_empty()
                || !native_quads(
                    tc.dataset.as_ref(),
                    None,
                    None,
                    Some(&absent),
                    GraphFilter::AnyGraph,
                )
                .is_empty();
            if mentioned {
                return Err(format!("the test graph mentions the harness's {absent}"));
            }
            absent
        }
    };

    let _function_scope =
        sparql::enter_function_scope(sparql::bind_in_current_env(&shapes.functions)?);
    let _aggregate_scope = sparql::enter_aggregate_scope(Arc::clone(&shapes.aggregates));
    let mut guard = RecursionGuard::new();
    eval_in_scope(&data, &focus, &expr, &mut guard, &tc.scope, Scope::EMPTY)
}

/// Grade one `sht:EvalNodeExpr` entry.
fn run_node_expr(id: &str, tc: &NodeExprCase) -> Result<(), String> {
    let produced = no_panic(|| eval_node_expr_case(tc))?;
    let expected = canonical_expectation(id, &tc.expected)?;
    compare_outputs(&produced, &expected, tc.ignore_order)
}

// ── sht:Infer ─────────────────────────────────────────────────────────────────

/// Run the rules; `Ok((inferred graph, projected base))`.
fn infer(tc: &InferCase) -> Result<(Arc<RdfDataset>, Arc<RdfDataset>), String> {
    let shapes_text = fs::read_to_string(&tc.shapes_path)
        .map_err(|e| format!("cannot read shapes {}: {e}", tc.shapes_path.display()))?;
    let text_ingest::TurtleDocument {
        dataset: shapes_dataset,
        prefixes: doc_prefixes,
        ..
    } = text_ingest::parse_turtle_document(&shapes_text, Some(&file_iri(&tc.shapes_path)))
        .map_err(|errors| format!("shapes graph parse error: {}", errors.join("; ")))?;
    let shapes = shapes::from_dataset_with_base(
        &shapes_dataset,
        None,
        &doc_prefixes,
        None,
        Some(tc.shapes_graph_iri.clone()),
        &shacl_corpora::w3c_case_imports(&shapes_dataset),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;
    let data_dataset = if tc.data_path == tc.shapes_path {
        shapes_dataset
    } else {
        parse_turtle_file(&tc.data_path).map_err(|e| format!("data graph parse error: {e}"))?
    };
    let projected = engine::project_dataset(data_dataset.as_ref())?;
    let holder = ShaclData::new(Arc::clone(&projected), Arc::clone(&projected), None);
    let inferred = apply_rules(&holder, &shapes).map_err(|e| format!("apply_rules failed: {e}"))?;
    Ok((inferred, projected))
}

/// `base ⊎ derived`, blank labels standardized apart per source.
fn merge(base: &RdfDataset, derived: &RdfDataset) -> Result<Arc<RdfDataset>, String> {
    let mut builder = RdfDatasetBuilder::new();
    builder.push_dataset(base);
    builder.push_dataset(derived);
    builder.freeze().map_err(|e| e.to_string())
}

/// The expected inferred triples as a dataset of their own.
fn expected_triples(expected: &InferExpected) -> Result<Arc<RdfDataset>, String> {
    match expected {
        InferExpected::Failure => Err("an sht:Failure entry has no expected triples".to_owned()),
        InferExpected::File(path) => parse_turtle_file(path),
        InferExpected::Triples(triples) => {
            // Each expected triple goes into the layer a parsed graph would put it in:
            // `r rdf:reifies <<( s p o )>>` declares a reifier, a triple about a reifier
            // is its annotation, anything else is a quad. Comparing an RDF 1.2 graph
            // under RDFC-1.0 compares those layers, so building every expected triple
            // as a quad would compare a graph with its reifications as plain triples
            // against the same graph parsed.
            let reifies = |p: &Term, o: &Term| {
                matches!(p, Term::NamedNode(p) if p.as_str() == RDF_REIFIES)
                    && matches!(o, Term::Triple(_))
            };
            let reifiers: Vec<&Term> = triples
                .iter()
                .filter(|[_, p, o]| reifies(p, o))
                .map(|[r, _, _]| r)
                .collect();
            let mut builder = RdfDatasetBuilder::new();
            for [s, p, o] in triples {
                let Term::NamedNode(predicate) = p else {
                    return Err(format!("expected triple has a non-IRI predicate {p}"));
                };
                if let Term::Triple(statement) = o
                    && reifies(p, o)
                {
                    builder.push_owned_reifier(&purrdf::RdfReifier::new(
                        s.to_rdf_term(),
                        purrdf::RdfTriple::new(
                            statement.subject.to_rdf_term(),
                            statement.predicate.as_str(),
                            statement.object.to_rdf_term(),
                        ),
                    ));
                } else if reifiers.contains(&s) {
                    builder.push_owned_annotation(&purrdf::RdfAnnotation::new(
                        s.to_rdf_term(),
                        predicate.as_str(),
                        o.to_rdf_term(),
                    ));
                } else {
                    builder.push_owned_quad(&RdfQuad::new(
                        s.to_rdf_term(),
                        predicate.as_str(),
                        o.to_rdf_term(),
                    ));
                }
            }
            builder.freeze().map_err(|e| e.to_string())
        }
    }
}

/// Grade one `sht:Infer` entry.
fn run_infer(tc: &InferCase) -> Result<(), String> {
    let outcome = no_panic(|| infer(tc));
    if matches!(tc.expected, InferExpected::Failure) {
        return match outcome {
            Err(_) => Ok(()),
            Ok(_) => Err("suite expects sht:Failure but the rules ran successfully".to_owned()),
        };
    }
    let (inferred, base) = outcome?;
    let expected_full = merge(base.as_ref(), expected_triples(&tc.expected)?.as_ref())?;
    let produced = canonicalize(inferred.as_ref()).nquads;
    let expected = canonicalize(expected_full.as_ref()).nquads;
    if produced == expected {
        return Ok(());
    }
    let produced_lines: Vec<&str> = produced.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    let missing: Vec<&str> = expected_lines
        .iter()
        .filter(|l| !produced_lines.contains(l))
        .copied()
        .collect();
    let extra: Vec<&str> = produced_lines
        .iter()
        .filter(|l| !expected_lines.contains(l))
        .copied()
        .collect();
    Err(format!(
        "inferred graph differs from data ⊎ expected (RDFC-1.0):\n  missing: {missing:?}\n  \
         extra: {extra:?}"
    ))
}

// ── srlt:* ────────────────────────────────────────────────────────────────────

/// Grade one SPARQL 1.2 RL entry, requiring every negative entry to fail at the stage
/// its type names — a negative syntax test in the parser, a negative well-formedness
/// test in the §4.2 check of a document that parses, a negative stratification test in
/// the §4.4 stratifier of a well-formed rule set — so a test cannot pass by failing for
/// some other reason. The suite README: "All the test are syntactically legal, i.e.
/// conform to the SPARQL-RL Rules grammar" (well-formedness), "All the test are
/// syntactically legal and well-formed" (stratification); and a positive syntax test is
/// "regardless of well-formedness and stratification".
///
/// An evaluation entry parses and checks the rule set, runs SPARQL 1.2 RL's infer
/// operation over `srlt:data`, and compares the INFERENCE graph — "The result of an
/// evaluation test is the inference graph" — with `mf:result` under RDF isomorphism
/// (RDFC-1.0 canonical N-Quads).
fn run_srl(tc: &SrlCase) -> Result<(), String> {
    let text = fs::read_to_string(&tc.ruleset)
        .map_err(|e| format!("cannot read {}: {e}", tc.ruleset.display()))?;
    let base = file_iri(&tc.ruleset);
    let parsed = srl::parse(&text, Some(&base));
    let expect_stage = |outcome: Result<(), SrlError>, stage: &str| -> Result<(), String> {
        match outcome {
            Err(error) => {
                let actual = srl_stage(&error);
                if actual == stage {
                    Ok(())
                } else {
                    Err(format!(
                        "expected a {stage} failure, but the {actual} stage refused: {error}"
                    ))
                }
            }
            Ok(()) => Err(format!(
                "expected a {stage} failure, but every stage accepted it"
            )),
        }
    };
    match tc.kind {
        SrlKind::PositiveSyntax => parsed.map(drop).map_err(|e| e.to_string()),
        SrlKind::NegativeSyntax => expect_stage(parsed.map(drop), "syntax"),
        SrlKind::PositiveWellFormedness => parsed
            .and_then(|document| document.check_well_formed())
            .map_err(|e| e.to_string()),
        SrlKind::NegativeWellFormedness => {
            let document = parsed.map_err(|e| format!("the document must parse: {e}"))?;
            expect_stage(document.check_well_formed(), "well-formedness")
        }
        SrlKind::PositiveStratification => {
            let document = parsed.map_err(|e| e.to_string())?;
            document.check_well_formed().map_err(|e| e.to_string())?;
            document.stratify().map(drop).map_err(|e| e.to_string())
        }
        SrlKind::NegativeStratification => {
            let document = parsed.map_err(|e| format!("the document must parse: {e}"))?;
            document
                .check_well_formed()
                .map_err(|e| format!("the rule set must be well formed: {e}"))?;
            expect_stage(document.stratify().map(drop), "stratification")
        }
        SrlKind::Eval => {
            let document = srl::parse_and_check(&text, Some(&base)).map_err(|e| e.to_string())?;
            let (Some(data), Some(result)) = (&tc.data, &tc.result) else {
                return Err("an evaluation entry has srlt:data and mf:result".to_owned());
            };
            let data = parse_turtle_file(data)?;
            let inference = no_panic(|| {
                srl::infer(&document, &data, &srl::InferOptions::default())
                    .map_err(|e| e.to_string())
            })?;
            let produced =
                expected_triples(&InferExpected::Triples(inference.inferred().to_vec()))?;
            let expected = parse_turtle_file(result)?;
            let produced = canonicalize(produced.as_ref()).nquads;
            let expected = canonicalize(expected.as_ref()).nquads;
            if produced == expected {
                return Ok(());
            }
            Err(format!(
                "inference graph differs from mf:result (RDFC-1.0):\n  produced:\n{produced}\n  \
                 expected:\n{expected}"
            ))
        }
    }
}

/// The preparation stage an SRL error comes from.
fn srl_stage(error: &SrlError) -> &'static str {
    match error {
        SrlError::Syntax { .. } => "syntax",
        SrlError::WellFormedness { .. } => "well-formedness",
        SrlError::Stratification { .. } => "stratification",
        SrlError::Import { .. } => "import",
        _ => "evaluation",
    }
}

// ── The harness ───────────────────────────────────────────────────────────────

/// Grade one entry.
fn run(case: &Case12) -> Result<(), String> {
    match &case.body {
        Body::Validate(tc) => match shacl_corpora::refused_import(&case.id) {
            Some(iris) => grade_refused_import(tc, iris),
            None => run_validate(&case.id, case.listed, tc),
        },
        Body::NodeExpr(tc) => run_node_expr(&case.id, tc),
        Body::Infer(tc) => run_infer(tc),
        Body::Srl(tc) => run_srl(tc),
    }
}

/// The label an approved entry that agrees with the harness is reported under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Category {
    /// Graded against its approved expectation, exactly, and agrees.
    Pass,
    /// An upstream erratum: [`NON_CANONICAL_EXPECTATIONS`]. Graded exactly
    /// against the canonical form of its one non-canonical expected literal, and
    /// agrees with THAT — never counted as a pass of the approved expectation.
    UpstreamErratum,
    /// Refused: its shapes graph imports an ontology no document can be supplied
    /// for (`shacl_corpora::REFUSED_UNRESOLVABLE_IMPORT`), and the load refused it
    /// with exactly `ShapesImportError::Unresolved` naming exactly that import.
    RefusedImport,
}

impl Category {
    /// The category an approved entry is reported under when it agrees.
    fn of(case: &Case12) -> Self {
        if shacl_corpora::refused_import(&case.id).is_some() {
            Self::RefusedImport
        } else if NON_CANONICAL_EXPECTATIONS
            .iter()
            .any(|(id, ..)| *id == case.id)
        {
            Self::UpstreamErratum
        } else {
            Self::Pass
        }
    }
}

/// Per-section and per-type counts.
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    passed: usize,
    errata: usize,
    refused: usize,
    xfailed: usize,
}

impl Tally {
    fn line(self) -> String {
        format!(
            "passed {:>3}  upstream-errata {:>2}  refused-unresolvable-import {:>2}  \
             xfailed {:>3}",
            self.passed, self.errata, self.refused, self.xfailed
        )
    }
}

/// The SHACL 1.2 entries graded as an expected refusal of an unresolvable import.
const W3C12_REFUSED_IMPORTS: usize = 1;

#[test]
fn w3c_shacl12_conformance() {
    let cases = shacl12_cases();
    assert_eq!(
        cases.len(),
        W3C12_TOTAL_CASES,
        "discovered SHACL 1.2 test count"
    );

    let xfail: BTreeMap<&str, &str> = XFAIL.iter().copied().collect();
    assert_eq!(
        xfail.len(),
        XFAIL.len(),
        "duplicate entries in the XFAIL ledger"
    );
    for (id, _) in XFAIL {
        assert!(
            cases.iter().any(|c| c.id == *id),
            "XFAIL ledger names unknown test {id} — stale entry?"
        );
    }

    let mut errors: Vec<String> = Vec::new();
    let mut sections: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut types: BTreeMap<String, Tally> = BTreeMap::new();
    let mut total = Tally::default();

    // Engine panics are caught by `no_panic` and graded as failures; the default
    // hook's backtraces would only drown the scoreboard.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let mut unlisted = 0usize;
    for case in &cases {
        if !case.listed {
            // Graded and reported by `w3c_shacl12_unlisted_vendored_files`.
            unlisted += 1;
            continue;
        }
        let bump = |tally: &mut Tally, f: fn(&mut Tally) -> &mut usize| *f(tally) += 1;
        let slot: Option<fn(&mut Tally) -> &mut usize> =
            match (run(case), xfail.get(case.id.as_str())) {
                (Ok(()), None) => Some(match Category::of(case) {
                    Category::Pass => |t| &mut t.passed,
                    Category::UpstreamErratum => |t| &mut t.errata,
                    Category::RefusedImport => |t| &mut t.refused,
                }),
                (Err(_), Some(_)) => Some(|t| &mut t.xfailed),
                (Ok(()), Some(reason)) => {
                    errors.push(format!(
                        "XPASS [{id}]: now passes — remove it from the XFAIL ledger (reason \
                         was: {reason})",
                        id = case.id
                    ));
                    None
                }
                (Err(e), None) => {
                    errors.push(format!("FAIL [{id}]: {e}", id = case.id));
                    None
                }
            };
        if let Some(slot) = slot {
            bump(sections.entry(case.section.as_str()).or_default(), slot);
            bump(types.entry(case.type_label()).or_default(), slot);
            bump(&mut total, slot);
        }
    }

    std::panic::set_hook(default_hook);

    println!(
        "W3C SHACL 1.2 conformance scoreboard ({} approved tests; {unlisted} entries of \
         unlisted vendored files reported apart):",
        cases.len() - unlisted
    );
    for (section, tally) in &sections {
        println!("  {section:<36} {}", tally.line());
    }
    println!("  by test type:");
    for (ty, tally) in &types {
        println!("  {ty:<36} {}", tally.line());
    }
    println!(
        "  W3C12 TOTAL: passed {}, upstream-errata {}, refused-unresolvable-import {}, \
         xfailed {}, ledger {}",
        total.passed,
        total.errata,
        total.refused,
        total.xfailed,
        XFAIL.len()
    );

    assert!(
        errors.is_empty(),
        "w3c_shacl12_conformance: {} error(s):\n{}",
        errors.len(),
        errors.join("\n\n")
    );
    assert_eq!(
        total.xfailed,
        XFAIL.len(),
        "xfail count must match the ledger exactly"
    );
    assert_eq!(
        total.errata, NON_CANONICAL_EXPECTATIONS_COUNT,
        "every upstream erratum is reported as one, and nothing else is"
    );
    assert_eq!(
        total.refused, W3C12_REFUSED_IMPORTS,
        "every expected import refusal is reported as one, and nothing else is"
    );
    assert_eq!(
        unlisted, W3C12_UNLISTED_ENTRIES,
        "entries of unlisted vendored files"
    );
    assert_eq!(
        total.passed + total.errata + total.refused + total.xfailed,
        W3C12_TOTAL_CASES - W3C12_UNLISTED_ENTRIES,
        "every approved test must be a pass, an upstream erratum, an expected import \
         refusal or a ledgered xfail"
    );
}

// ── Unlisted vendored files ───────────────────────────────────────────────────

/// The number of entries in vendored files that no upstream manifest includes
/// (`shacl12::W3C12_UNINCLUDED_MANIFESTS`): `core/node/xone-002`,
/// `core/node/xone-003` and `inference-rules/rdfs/rectangle-condition`.
const W3C12_UNLISTED_ENTRIES: usize = 3;

/// The entries of vendored files that no upstream manifest includes, graded
/// exactly against their own file — `core/node/xone-003` with the one proven
/// delta in [`UNLISTED_FILE_DELTAS`], every other one as written — and reported
/// under their own label. They are not the approved suite and are never counted
/// among its passes. Every one must pass; there is no ledger here.
#[test]
fn w3c_shacl12_unlisted_vendored_files() {
    let cases = shacl12_cases();
    let unlisted: Vec<&Case12> = cases.iter().filter(|c| !c.listed).collect();
    assert_eq!(unlisted.len(), W3C12_UNLISTED_ENTRIES, "unlisted entries");
    for (id, ..) in UNLISTED_FILE_DELTAS {
        assert!(
            unlisted.iter().any(|c| c.id == *id),
            "UNLISTED_FILE_DELTAS names {id}, which is not an entry of an unlisted file"
        );
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut errors: Vec<String> = Vec::new();
    let (mut exact, mut with_delta) = (0usize, 0usize);
    for case in &unlisted {
        match run(case) {
            Ok(()) if UNLISTED_FILE_DELTAS.iter().any(|(id, ..)| *id == case.id) => {
                with_delta += 1;
            }
            Ok(()) => exact += 1,
            Err(e) => errors.push(format!("FAIL [{id}]: {e}", id = case.id)),
        }
    }
    std::panic::set_hook(default_hook);

    println!("W3C SHACL 1.2 entries of unlisted vendored files:");
    for case in &unlisted {
        println!("  {:<44} {}", case.id, case.type_label());
    }
    println!(
        "  W3C12 UNLISTED: passed {}, exact {exact}, with-delta {with_delta}, total {}",
        exact + with_delta,
        unlisted.len()
    );
    assert!(
        errors.is_empty(),
        "w3c_shacl12_unlisted_vendored_files: {} error(s):\n{}",
        errors.len(),
        errors.join("\n\n")
    );
    assert_eq!(
        with_delta, UNLISTED_FILE_DELTAS_COUNT,
        "entries graded with a delta"
    );
}

/// No approved entry is graded against an amended expectation: naming a LISTED
/// entry in the delta table is refused, and the neighbouring unlisted entry the
/// table really names is graded through its delta.
#[test]
fn a_listed_entry_is_never_amended() {
    let cases = shacl12_cases();
    let (id, ..) = UNLISTED_FILE_DELTAS[0];
    let case = cases
        .iter()
        .find(|c| c.id == id)
        .expect("the delta's entry is discovered");
    let Body::Validate(tc) = &case.body else {
        panic!("{id} is an sht:Validate test");
    };
    assert!(!case.listed, "{id} is an entry of an unlisted file");
    assert!(
        graded_expectation(id, false, tc).is_ok(),
        "the unlisted entry is graded through its delta"
    );
    let error = graded_expectation(id, true, tc)
        .err()
        .expect("the same entry, were it listed, is refused");
    assert!(error.contains("an upstream manifest lists"), "{error}");
}

// ── The completion gate ───────────────────────────────────────────────────────

/// The xfail ledger is empty: every discovered SHACL 1.2 entry passes. An entry
/// the engine cannot pass is a defect to fix, never a row to add here.
#[test]
fn xfail_ledger_is_empty() {
    assert!(
        XFAIL.is_empty(),
        "the SHACL 1.2 xfail ledger must be empty, but it names {} entr(y/ies): {:?}",
        XFAIL.len(),
        XFAIL.iter().map(|(id, _)| *id).collect::<Vec<_>>()
    );
}

/// Every entry the engine was measured failing before the SHACL 1.2 work, by
/// its discovered id. Each must be discovered under exactly this id (a name that
/// is not discovered fails, so a typo cannot hide an entry) and must pass under
/// the harness's grading — which applies [`UNLISTED_FILE_DELTAS`] (to an entry
/// of an unlisted file) and [`NON_CANONICAL_EXPECTATIONS`]. Passing here is a
/// grading verdict, not a count: which category each entry is REPORTED under is
/// [`w3c_shacl12_conformance`]'s and [`w3c_shacl12_unlisted_vendored_files`]'s
/// business.
const INVENTORY: &[&str] = &[
    // Core list components.
    "core/node/minListLength-001",
    "core/node/maxListLength-001",
    "core/property/minListLength-001",
    "core/property/maxListLength-001",
    "core/node/in-003",
    "core/node/xone-003",
    "core/node/uniqueMembers-001",
    "core/property/uniqueMembers-001",
    "core/node/memberShape-001",
    "core/property/memberShape-001",
    // Other new core components.
    "core/node/uniqueValuesFor-001",
    "core/node/uniqueValuesFor-002",
    "core/node/uniqueValuesFor-003",
    "core/node/uniqueValuesFor-005",
    "core/property/subsetOf-001",
    "core/property/subsetOf-002",
    "core/property/someValue-001",
    "core/property/rootClass-001",
    "core/property/singleLine-001",
    // List-valued parameters.
    "core/node/datatype-003",
    "core/property/datatype-004",
    "core/property/class-002",
    "core/node/nodeKind-002",
    // Closed shapes.
    "core/node/closed-003",
    "core/node/closed-004",
    // Path-valued property pairs.
    "core/property/equals-002",
    "core/property/disjoint-002",
    "core/property/lessThan-003",
    "core/property/lessThanOrEquals-002",
    // Reifier annotations, Debug and Trace severities.
    "core/misc/deactivated-003",
    "core/misc/severity-003",
    "core/misc/severity-004",
    "core/misc/severity-005",
    // Report details.
    "core/validation-reports/conformance-disallows-001",
    "core/property/reifierShape-001",
    "core/property/reifierShape-002",
    "core/property/uniqueLang-003",
    // Targets.
    "core/targets/shape-001",
    "core/targets/targetClassImplicit-002",
    "core/targets/targetWhere-001",
    "sparql/targets/targetNode-select-001",
    // SPARQL surface.
    "sparql/property/property-select-001",
    "sparql/property/property-sparqlExpr-001",
    "sparql/node/prefixes-002",
    "sparql/functions/instanceCount-example",
    // Inference rules, validated.
    "inference-rules/rules-entailment-validation",
    // Node expressions.
    "node-expr/shnex-sparql/plus-example",
    "node-expr/shnex-sparql/encode-example",
    "node-expr/shnex-sparql/bound-example",
    "node-expr/shnex-sparql/coalesce-example",
    "node-expr/shnex/filterShape-integers",
    "node-expr/shnex/filterShape-unconstrained",
    "node-expr/shnex/distinct-list",
    "node-expr/shnex/distinct-termEquality",
    "node-expr/shnex/findFirst-empty",
    "node-expr/shnex/matchAll-empty",
    "node-expr/shnex/orderBy-height",
    "node-expr/shnex/var-bound",
    "node-expr/shnex-sparql/ceil-example",
    "node-expr/shnex-sparql/floor-example",
    "node-expr/shnex-sparql/round-example",
    "node-expr/shnex-sparql/divide-example",
    "node-expr/shnex-sparql/seconds-example",
    "node-expr/shnex/sum-totalRevenue",
];

/// Every `sht:Infer` entry, by name. [`every_planned_inventory_id_passes`] also
/// proves this is exactly the discovered set of `sht:Infer` entries.
const INFER_INVENTORY: &[&str] = &[
    "inference-rules/SPARQLRuleTemplate-example-Multiply",
    "inference-rules/SPARQLRuleTemplate-example-SymmetricProperty",
    "inference-rules/SPARQLRuleTemplate-missing-param",
    "inference-rules/TripleRule-example-childCount",
    "inference-rules/TripleRule-example-squares",
    "inference-rules/expectedPredicate-example",
    "inference-rules/global-symmetric",
    "inference-rules/layers-example",
    "inference-rules/rdfs/rdfs-domain-1",
    "inference-rules/rdfs/rdfs-domain-2",
    "inference-rules/rdfs/rdfs-range-1",
    "inference-rules/rdfs/rdfs-range-2",
    "inference-rules/rdfs/rdfs-subclass-1",
    "inference-rules/rdfs/rdfs-subproperty-1",
    "inference-rules/rdfs/rectangle-condition",
    "inference-rules/rectangle-condition",
    "inference-rules/rectangle-deactivated",
    "inference-rules/rectangle-order",
    "inference-rules/rectangle-prefixes",
    "inference-rules/rectangle-simple",
    "inference-rules/ruleProcessor-unknown-at-rule",
    "inference-rules/ruleProcessor-unknown-at-ruleset",
    "inference-rules/run-once-blank-node-feed",
    "inference-rules/run-once-example",
    "inference-rules/same-order",
    "inference-rules/temp-triples-example",
    "inference-rules/unknown-rule-type",
];

/// The number of discovered SPARQL 1.2 RL (`srlt:*`) entries, every one of which
/// must pass. Pinned so a walk that stops reaching some of them cannot pass by
/// grading fewer.
const SRL_TOTAL: usize = 203;

/// Every id in the measured failure inventory is discovered and passes, every
/// `sht:Infer` entry passes by name, and every SPARQL 1.2 RL entry passes.
#[test]
fn every_planned_inventory_id_passes() {
    let cases = shacl12_cases();
    let by_id: BTreeMap<&str, &Case12> = cases.iter().map(|c| (c.id.as_str(), c)).collect();

    let named: Vec<&str> = INVENTORY.iter().chain(INFER_INVENTORY).copied().collect();
    let unique: std::collections::BTreeSet<&str> = named.iter().copied().collect();
    assert_eq!(unique.len(), named.len(), "an inventory id is listed twice");

    let discovered_infer: std::collections::BTreeSet<&str> = cases
        .iter()
        .filter(|c| matches!(c.body, Body::Infer(_)))
        .map(|c| c.id.as_str())
        .collect();
    let listed_infer: std::collections::BTreeSet<&str> = INFER_INVENTORY.iter().copied().collect();
    assert_eq!(
        listed_infer, discovered_infer,
        "INFER_INVENTORY must name exactly the discovered sht:Infer entries"
    );
    assert!(
        INFER_INVENTORY
            .iter()
            .all(|id| { by_id.get(id).is_some_and(|c| c.type_label() == "sht:Infer") }),
        "every INFER_INVENTORY entry is an sht:Infer test"
    );

    let srl: Vec<&Case12> = cases
        .iter()
        .filter(|c| matches!(c.body, Body::Srl(_)))
        .collect();
    assert_eq!(srl.len(), SRL_TOTAL, "discovered SPARQL 1.2 RL entry count");
    assert!(
        srl.iter().all(|c| c.id.starts_with("sparql-rl/")),
        "every SPARQL 1.2 RL entry lives under sparql-rl/"
    );

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut errors: Vec<String> = Vec::new();
    for id in &named {
        match by_id.get(id) {
            None => errors.push(format!("NOT DISCOVERED [{id}]: no entry has this id")),
            Some(case) => {
                if let Err(e) = run(case) {
                    errors.push(format!("FAIL [{id}]: {e}"));
                }
            }
        }
    }
    for case in &srl {
        if let Err(e) = run(case) {
            errors.push(format!("FAIL [{id}]: {e}", id = case.id));
        }
    }
    std::panic::set_hook(default_hook);

    assert!(
        errors.is_empty(),
        "every_planned_inventory_id_passes: {} error(s):\n{}",
        errors.len(),
        errors.join("\n\n")
    );
}

// ── The canonical-form table ──────────────────────────────────────────────────

/// Both directions of [`NON_CANONICAL_EXPECTATIONS`], plus its count pin.
///
/// For every entry: the named test exists and is a node-expression test; its
/// expected results contain a literal with the stated lexical form; the XSD 1.1
/// canonical mapping of that literal's value (`purrdf_xsd`) is EXACTLY the stated
/// canonical form; and that canonical form DIFFERS from the expected one — an
/// expectation that was already canonical has no business in the table. The
/// engine side (its output equals the canonical form, term for term) is graded
/// by the harness itself.
#[test]
fn non_canonical_expectations_are_really_non_canonical() {
    assert_eq!(
        NON_CANONICAL_EXPECTATIONS.len(),
        NON_CANONICAL_EXPECTATIONS_COUNT,
        "NON_CANONICAL_EXPECTATIONS count pin"
    );
    let cases = shacl12_cases();
    for (id, lexical, canonical, clause) in NON_CANONICAL_EXPECTATIONS {
        assert!(
            clause.starts_with("XSD 1.1 Part 2 §"),
            "{id}: the clause must cite XSD 1.1 Part 2, got {clause:?}"
        );
        assert_ne!(lexical, canonical, "{id}: an entry must change the form");
        let case = cases
            .iter()
            .find(|c| c.id == *id)
            .unwrap_or_else(|| panic!("{id}: no such test"));
        assert!(
            case.listed,
            "{id}: an upstream erratum is an entry of the approved suite"
        );
        let Body::NodeExpr(tc) = &case.body else {
            panic!("{id}: not an sht:EvalNodeExpr test");
        };
        let literal = tc
            .expected
            .iter()
            .find_map(|t| match t {
                Term::Literal(l) if l.value() == *lexical => Some(l),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{id}: no expected literal spelled {lexical:?}"));
        let value = purrdf_xsd::parse_by_iri(literal.value(), literal.datatype_str())
            .unwrap_or_else(|e| panic!("{id}: {lexical:?} is not a valid lexical form: {e}"))
            .unwrap_or_else(|| panic!("{id}: <{}> is not an XSD datatype", literal.datatype_str()));
        assert_eq!(
            value.canonical_lexical(),
            *canonical,
            "{id}: the XSD 1.1 canonical form of {lexical:?}^^<{}>",
            literal.datatype_str()
        );
    }
}

// ── The expectation-defect table ──────────────────────────────────────────────

/// Both directions of [`UNLISTED_FILE_DELTAS`], plus its count pin.
///
/// For every entry: the test exists, is an entry of a file no upstream manifest
/// includes, is an `sht:Validate` entry with an expected report, is not also
/// ledgered in [`XFAIL`], and the clause quotes SHACL 1.2 Core. Then, on the ENGINE'S REAL REPORT:
///
/// * it agrees with the amended expectation (the delta is exactly what the
///   engine does differently);
/// * it disagrees with the approved expectation (the delta is not vacuous);
///
/// and, as controls over the comparison itself, with the engine's `sh:conforms`
/// held fixed:
///
/// * a report MISSING the delta — the approved results verbatim — fails;
/// * a report carrying a DIFFERENT delta — the same result, every field the
///   delta changes given another value — fails;
/// * a report carrying the delta plus a difference in any field the delta holds
///   (focus node, path, value, component, severity or source shape, one at a
///   time) fails;
/// * a report carrying the delta PLUS an extra, different amendment of another
///   result fails.
#[test]
fn unlisted_file_deltas_are_exact() {
    assert_eq!(
        UNLISTED_FILE_DELTAS.len(),
        UNLISTED_FILE_DELTAS_COUNT,
        "UNLISTED_FILE_DELTAS count pin"
    );
    let cases = shacl12_cases();
    for (id, clause, deltas) in UNLISTED_FILE_DELTAS {
        assert!(
            clause.starts_with("SHACL 1.2 Core §") && clause.contains('"'),
            "{id}: the clause must quote SHACL 1.2 Core, got {clause:?}"
        );
        assert!(!deltas.is_empty(), "{id}: an entry must carry a delta");
        assert!(
            !XFAIL.iter().any(|(entry, _)| entry == id),
            "{id}: an expectation defect is graded, never also an expected failure"
        );
        let case = cases
            .iter()
            .find(|c| c.id == *id)
            .unwrap_or_else(|| panic!("{id}: no such test"));
        assert!(
            !case.listed,
            "{id}: a delta amends only an entry of a file no upstream manifest includes"
        );
        let Body::Validate(tc) = &case.body else {
            panic!("{id}: not an sht:Validate test");
        };
        let Expected::Report { conforms, results } = &tc.expected else {
            panic!("{id}: the approved result is sht:Failure");
        };
        let amended = amend(id, results, deltas).unwrap_or_else(|e| panic!("{e}"));
        assert_ne!(
            &amended, results,
            "{id}: the delta must change the expectation"
        );

        let (produced_conforms, produced) =
            produce(tc).unwrap_or_else(|e| panic!("{id}: the engine must validate: {e}"));
        let amended_expectation = Expected::Report {
            conforms: *conforms,
            results: amended.clone(),
        };
        let approved_expectation = Expected::Report {
            conforms: *conforms,
            results: results.clone(),
        };
        grade(
            &amended_expectation,
            Ok((produced_conforms, produced.clone())),
        )
        .unwrap_or_else(|e| panic!("{id}: the engine must equal the amended report: {e}"));
        assert!(
            grade(&approved_expectation, Ok((produced_conforms, produced))).is_err(),
            "{id}: the engine must differ from the approved report, or the entry is vacuous"
        );

        // A report missing the delta.
        assert!(
            grade(
                &amended_expectation,
                Ok((produced_conforms, results.clone()))
            )
            .is_err(),
            "{id}: a report missing the delta must fail"
        );

        // A report with a different delta in its place: every field the delta
        // changes carries another value instead.
        let differently: Vec<Amendment> = deltas
            .iter()
            .map(|delta| Amendment {
                graded: other_in_delta_fields(delta),
                ..*delta
            })
            .collect();
        let differently_amended = amend(id, results, &differently).expect("the same result");
        assert!(
            grade(
                &amended_expectation,
                Ok((produced_conforms, differently_amended))
            )
            .is_err(),
            "{id}: a report carrying a different delta must fail"
        );

        // A report with the delta AND a difference in a field the delta holds:
        // the entry licenses its delta and nothing else about that result.
        for (field, held) in deltas.iter().flat_map(|delta| {
            held_field_variants(delta)
                .into_iter()
                .map(move |(field, graded)| (field, Amendment { graded, ..*delta }))
        }) {
            let widened = amend(id, results, &[held]).expect("the same result");
            assert!(
                grade(&amended_expectation, Ok((produced_conforms, widened))).is_err(),
                "{id}: a report carrying the delta plus a different {field} must fail"
            );
        }

        // A report with the delta plus an extra, different result.
        let mut extra = amended;
        *extra
            .entry((
                "<http://example.org/ns#extra-focus>".to_owned(),
                Some("<http://example.org/ns#extra-path>".to_owned()),
                None,
                "<http://www.w3.org/ns/shacl#MinCountConstraintComponent>".to_owned(),
                "<http://www.w3.org/ns/shacl#Violation>".to_owned(),
                "<http://example.org/ns#extra-shape>".to_owned(),
            ))
            .or_insert(0) += 1;
        assert!(
            grade(&amended_expectation, Ok((produced_conforms, extra))).is_err(),
            "{id}: a report carrying the delta plus another difference must fail"
        );
    }
}

/// A value no vendored report carries, standing in for "some other term".
const ELSEWHERE: &str = "<http://example.org/ns#elsewhere>";

/// `delta`'s graded tuple with every field the delta CHANGES replaced by
/// [`ELSEWHERE`]: the same result, amended differently.
fn other_in_delta_fields(delta: &Amendment) -> ResultTuple {
    let (approved, graded) = (delta.approved, delta.graded);
    ResultTuple {
        focus: if approved.focus == graded.focus {
            graded.focus
        } else {
            ELSEWHERE
        },
        path: if approved.path == graded.path {
            graded.path
        } else {
            Some(ELSEWHERE)
        },
        value: if approved.value == graded.value {
            graded.value
        } else {
            Some(ELSEWHERE)
        },
        component: if approved.component == graded.component {
            graded.component
        } else {
            ELSEWHERE
        },
        severity: if approved.severity == graded.severity {
            graded.severity
        } else {
            ELSEWHERE
        },
        source_shape: if approved.source_shape == graded.source_shape {
            graded.source_shape
        } else {
            ELSEWHERE
        },
    }
}

/// One variant of `delta`'s graded tuple per field the delta HOLDS, that field
/// replaced by [`ELSEWHERE`], named for the assertion message.
fn held_field_variants(delta: &Amendment) -> Vec<(&'static str, ResultTuple)> {
    let (approved, graded) = (delta.approved, delta.graded);
    let mut variants = Vec::new();
    if approved.focus == graded.focus {
        variants.push((
            "focus node",
            ResultTuple {
                focus: ELSEWHERE,
                ..graded
            },
        ));
    }
    if approved.path == graded.path {
        variants.push((
            "result path",
            ResultTuple {
                path: Some(ELSEWHERE),
                ..graded
            },
        ));
    }
    if approved.value == graded.value {
        variants.push((
            "value",
            ResultTuple {
                value: Some(ELSEWHERE),
                ..graded
            },
        ));
    }
    if approved.component == graded.component {
        variants.push((
            "component",
            ResultTuple {
                component: ELSEWHERE,
                ..graded
            },
        ));
    }
    if approved.severity == graded.severity {
        variants.push((
            "severity",
            ResultTuple {
                severity: ELSEWHERE,
                ..graded
            },
        ));
    }
    if approved.source_shape == graded.source_shape {
        variants.push((
            "source shape",
            ResultTuple {
                source_shape: ELSEWHERE,
                ..graded
            },
        ));
    }
    variants
}

/// A delta naming a result the approved report does not hold is refused as a
/// stale entry rather than amending a result into existence; the neighbouring
/// delta that names a real result applies.
#[test]
fn a_stale_unlisted_file_delta_is_refused() {
    let focus = "<http://example.org/ns#focus>";
    let component = "<http://www.w3.org/ns/shacl#MinListLengthConstraintComponent>";
    let severity = "<http://www.w3.org/ns/shacl#Violation>";
    let approved = ResultTuple {
        focus,
        path: None,
        value: None,
        component,
        severity,
        source_shape: "<http://example.org/ns#shape>",
    };
    let mut results = Multiset::new();
    results.insert(approved.key(), 1);
    let real = Amendment {
        approved,
        graded: ResultTuple {
            path: Some("<http://example.org/ns#p>"),
            ..approved
        },
    };
    let stale = Amendment {
        approved: ResultTuple {
            focus: ELSEWHERE,
            ..approved
        },
        ..real
    };
    let amended = amend("control", &results, &[real]).expect("a real result is amended");
    assert_eq!(
        amended.keys().next().and_then(|tuple| tuple.1.as_deref()),
        Some("<http://example.org/ns#p>")
    );
    let error = amend("control", &results, &[stale]).expect_err("a stale delta is refused");
    assert!(error.contains("stale entry"), "{error}");
}

// ── Comparator controls ───────────────────────────────────────────────────────

fn xsd(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://www.w3.org/2001/XMLSchema#{local}"))
}

fn typed(lexical: &str, datatype: &str) -> Term {
    Term::Literal(Literal::new_typed_literal(lexical, xsd(datatype)))
}

/// The comparator is RDF 1.2 term equality: the same term agrees, a different
/// value disagrees, the same lexical form under another datatype disagrees, and
/// the same value under another lexical form disagrees.
#[test]
fn comparator_is_exact_term_equality() {
    let four = typed("4", "decimal");
    assert!(
        compare_outputs(
            std::slice::from_ref(&four),
            std::slice::from_ref(&four),
            false
        )
        .is_ok()
    );
    assert!(
        compare_outputs(&[typed("5", "decimal")], std::slice::from_ref(&four), false).is_err(),
        "a different value must not agree"
    );
    assert!(
        compare_outputs(&[typed("4", "integer")], std::slice::from_ref(&four), false).is_err(),
        "a different datatype must not agree"
    );
    assert!(
        compare_outputs(&[typed("4.0", "decimal")], &[four], false).is_err(),
        "a different lexical form of the same value must not agree"
    );
}

/// Order matters unless the entry says otherwise, and cardinality always does.
#[test]
fn comparator_order_and_cardinality() {
    let (a, b) = (typed("1", "integer"), typed("2", "integer"));
    assert!(compare_outputs(&[b.clone(), a.clone()], &[a.clone(), b.clone()], false).is_err());
    assert!(compare_outputs(&[b.clone(), a.clone()], &[a.clone(), b.clone()], true).is_ok());
    assert!(
        compare_outputs(&[a.clone(), a.clone(), b.clone()], &[a, b], true).is_err(),
        "ignoring order must not ignore cardinality"
    );
}

/// The canonical-form substitution touches only a test the table names: every
/// other test keeps its expectation exactly as the suite spells it.
#[test]
fn canonical_substitution_is_scoped_to_the_table() {
    let spelled = [typed("4.0", "decimal")];
    let kept = canonical_expectation("node-expr/not-in-the-table", &spelled)
        .expect("an unlisted test passes through");
    assert_eq!(kept, spelled);
    for (id, lexical, canonical, _) in NON_CANONICAL_EXPECTATIONS {
        let expected = [typed(lexical, "decimal"), typed("7", "integer")];
        let swapped = canonical_expectation(id, &expected).expect("a listed test substitutes");
        assert_eq!(
            swapped,
            [typed(canonical, "decimal"), typed("7", "integer")]
        );
        assert!(
            canonical_expectation(id, &[typed("7", "integer")]).is_err(),
            "{id}: an entry matching no expected literal is stale"
        );
    }
}

/// PurRDF's own numeric output is the XSD 1.1 canonical lexical form: the
/// `xsd:decimal` value four is spelled `"4"`, not `"4.0"` (XSD 1.1 Part 2
/// §3.3.3.1 and §E.1 `decimalCanonicalMap`). Evaluated through the same library
/// path the harness grades with, on an `example.org` fixture.
#[test]
fn engine_emits_the_canonical_decimal_lexical_form() {
    let ttl = r"
        @prefix ex: <http://example.org/ns#> .
        @prefix sparql: <http://www.w3.org/ns/sparql#> .
        ex:root ex:expr [ sparql:ceil ( 3.2 ) ] .
    ";
    let dataset = purrdf::parse_dataset(ttl.as_bytes(), "text/turtle", None).expect("parse");
    let root = native_quads(
        dataset.as_ref(),
        None,
        Some(&Term::NamedNode(NamedNode::new_unchecked(
            "http://example.org/ns#expr",
        ))),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(_, _, o)| o)
    .next()
    .expect("the fixture's expression node");
    let (shapes, exprs) = shapes::from_dataset_with_node_expressions(
        &dataset,
        &[],
        None,
        std::slice::from_ref(&root),
        &purrdf_shapes::ShapesImports::new(),
    )
    .expect("the expression parses");
    let projected = engine::project_dataset(dataset.as_ref()).expect("projection");
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    let _function_scope =
        sparql::enter_function_scope(sparql::bind_in_current_env(&shapes.functions).expect("bind"));
    let mut guard = RecursionGuard::new();
    let out = eval_node_expr_in_scope(
        &data,
        &Term::blank(ABSENT_FOCUS),
        &exprs[0],
        &mut guard,
        Scope::EMPTY,
    )
    .expect("evaluates");
    assert_eq!(out, [typed("4", "decimal")]);
}

/// The shared grader grades SHACL-SPARQL result annotations: a first-party case in
/// the suite's own form whose expected result carries `ex:label "urgent"` passes as
/// written, and fails once the expected annotation value is altered —
/// so an engine that dropped or corrupted annotations could not pass a suite case
/// that states one.
#[test]
fn the_grader_grades_result_annotations_both_ways() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/result-annotation-grading.ttl");
    let root = fixture
        .parent()
        .expect("the fixture has a directory")
        .to_path_buf();
    let mut cases: Vec<W3cCase> = Vec::new();
    shacl_corpora::walk_manifest(&fixture, &mut |g, entry, manifest_path| {
        cases.extend(shacl_corpora::parse_entry(g, entry, manifest_path, &root));
    });
    let [case] = cases.as_mut_slice() else {
        panic!("the fixture holds exactly one sht:Validate case");
    };
    assert_eq!(
        case.expected_annotations.len(),
        1,
        "the harness reads the expected result's annotation"
    );
    shacl_corpora::report_grading::run_validate_case(case)
        .expect("the engine carries the expected annotation");

    let (_, pairs) = &mut case.expected_annotations[0];
    *pairs = std::collections::BTreeSet::from([(
        "<http://example.org/ns#label>".to_owned(),
        "\"routine\"".to_owned(),
    )]);
    assert!(
        shacl_corpora::report_grading::run_validate_case(case).is_err(),
        "a wrong expected annotation value must fail the case"
    );
}

/// The shared grader grades `sh:sourceShape`: the first-party annotation case passes
/// as written (its expected result names `ex:S`), and fails once the expected
/// result names another shape with every other field held — so an engine that
/// attributed a result to the wrong shape could not pass a suite case.
#[test]
fn the_grader_grades_the_source_shape_both_ways() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/result-annotation-grading.ttl");
    let root = fixture
        .parent()
        .expect("the fixture has a directory")
        .to_path_buf();
    let mut cases: Vec<W3cCase> = Vec::new();
    shacl_corpora::walk_manifest(&fixture, &mut |g, entry, manifest_path| {
        cases.extend(shacl_corpora::parse_entry(g, entry, manifest_path, &root));
    });
    let [case] = cases.as_mut_slice() else {
        panic!("the fixture holds exactly one sht:Validate case");
    };
    shacl_corpora::report_grading::run_validate_case(case)
        .expect("the engine names the expected source shape");

    let Expected::Report { results, .. } = &mut case.expected else {
        panic!("the fixture expects a report");
    };
    let (tuple, count) = results.pop_first().expect("the fixture expects one result");
    assert_eq!(
        tuple.5, "<http://example.org/ns#S>",
        "the expected source shape"
    );
    let mut elsewhere = tuple.clone();
    elsewhere.5 = ELSEWHERE.to_owned();
    results.insert(elsewhere, count);
    // The annotation claim is keyed by the same tuple; move it with the result so
    // only the source shape differs.
    for (claimed, _) in &mut case.expected_annotations {
        if *claimed == tuple {
            claimed.5 = ELSEWHERE.to_owned();
        }
    }
    assert!(
        shacl_corpora::report_grading::run_validate_case(case).is_err(),
        "a different expected source shape must fail the case"
    );
}

/// The shared grader grades `sh:detail` exactly where the approved report states
/// it, and not where it states none.
///
/// Over the approved `core/node/memberShape-001` — whose `ex:list2` result states
/// the one detail `"Bob"` and whose `ex:list5` result states exactly `"Charlie"`
/// and `"Donna"` — the case passes as written, and fails when the stated details
/// lose one, gain one, or change one field of one. Then the control: with
/// `ex:list5`'s stated details removed from the expectation, the case still
/// passes while the engine's own `ex:list5` result carries two details — a
/// produced detail the expectation does not spell is not graded (SHACL 1.2 Core
/// §6.7.2.6, quoted in the grader's docs).
#[test]
fn the_grader_grades_sh_detail_where_it_is_stated() {
    use shacl_corpora::ExpectedResult;

    let mut cases = shacl12_cases();
    let case = cases
        .iter_mut()
        .find(|c| c.id == "core/node/memberShape-001")
        .expect("memberShape-001 is discovered");
    let Body::Validate(tc) = &mut case.body else {
        panic!("memberShape-001 is an sht:Validate test");
    };
    let list5 = "<http://example.com/ns#list5>";
    let stated: Vec<(String, usize)> = tc
        .expected_details
        .iter()
        .map(|e| (e.tuple.0.clone(), e.details.as_ref().map_or(0, Vec::len)))
        .collect();
    assert_eq!(
        stated,
        [
            ("<http://example.com/ns#list2>".to_owned(), 1),
            (list5.to_owned(), 2)
        ],
        "the approved report states details on ex:list2 and ex:list5"
    );
    shacl_corpora::report_grading::run_validate_case(tc)
        .expect("the engine carries exactly the stated details");

    let original = tc.expected_details.clone();
    let list5_at = original
        .iter()
        .position(|e| e.tuple.0 == list5)
        .expect("ex:list5 states details");
    let with_list5 = |details: Option<Vec<ExpectedResult>>| {
        let mut amended = original.clone();
        amended[list5_at].details = details;
        amended
    };
    let stated5 = original[list5_at]
        .details
        .clone()
        .expect("ex:list5 states details");

    let mut lost = stated5.clone();
    lost.pop();
    let mut gained = stated5.clone();
    gained.push(stated5[0].clone());
    let mut changed = stated5.clone();
    changed[1].tuple.5 = ELSEWHERE.to_owned();
    for (what, details) in [("loses", lost), ("gains", gained), ("changes", changed)] {
        tc.expected_details = with_list5(Some(details));
        assert!(
            shacl_corpora::report_grading::run_validate_case(tc).is_err(),
            "an expectation that {what} one of ex:list5's details must fail"
        );
    }

    tc.expected_details = with_list5(None);
    let report = shacl_corpora::report_grading::validate_case(tc).expect("the engine validates");
    let produced5 = report
        .results
        .iter()
        .find(|r| shacl_corpora::norm(&r.focus_node) == list5)
        .expect("the engine reports ex:list5");
    assert_eq!(
        produced5.details.len(),
        2,
        "the engine's ex:list5 result carries its two details"
    );
    shacl_corpora::report_grading::run_validate_case(tc)
        .expect("a produced detail the expectation does not state is not graded");
}

/// The expected import refusal is EXACT in both directions. `validator-001`
/// refused for exactly DASH passes; the same case expected to be refused for a
/// different import fails; and a case whose shapes graph LOADS — here the
/// neighbouring `sparql/component/optional-001`, which imports nothing
/// unresolvable — fails when graded as a refusal, so a surprise load success can
/// never read as the refusal the category reports.
#[test]
fn the_expected_import_refusal_is_exact() {
    let cases = shacl12_cases();
    let validate = |id: &str| -> &W3cCase {
        match &cases
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("{id} is discovered"))
            .body
        {
            Body::Validate(tc) => tc,
            _ => panic!("{id} is an sht:Validate test"),
        }
    };
    let refused = validate("sparql/component/validator-001");
    grade_refused_import(refused, &[shacl_corpora::DASH]).expect("refused for exactly DASH");
    assert!(
        grade_refused_import(refused, &["http://example.org/ns#other"]).is_err(),
        "a refusal naming another import is not this refusal"
    );
    let loads = validate("sparql/component/optional-001");
    shacl_corpora::report_grading::run_validate_case(loads)
        .expect("the neighbouring case loads and agrees with its report");
    let error = grade_refused_import(loads, &[shacl_corpora::DASH])
        .expect_err("a case that loads is not an import refusal");
    assert!(error.contains("LOADED"), "{error}");
}
