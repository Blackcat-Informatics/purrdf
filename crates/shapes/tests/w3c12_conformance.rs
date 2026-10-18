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
//!   the entry sets `sht:ignoreOrder true`, in which case as a multiset. There is
//!   no other departure from exact equality.
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

#[path = "support/report.rs"]
mod report;
#[path = "support/w3c_earl.rs"]
mod w3c_earl;

use purrdf_shapes::shacl_corpora;
use report::merge;

use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use purrdf_rdf::{RdfDataset, RdfDatasetBuilder, RdfQuad, canonicalize};
use purrdf_shapes::data::{GraphFilter, ShaclData, native_quads};
use purrdf_shapes::expression::{
    Binding, NodeExpr, RecursionGuard, Scope, eval_node_expr_in_scope,
};
use purrdf_shapes::srl::{self, SrlError};
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_shapes::{ShaclProfile, apply_rules, engine, shapes, sparql, text_ingest};

use shacl_corpora::node_expr_grading::{ABSENT_FOCUS, compare_outputs};
use shacl_corpora::report_grading::{grade, grade_against, no_panic, produce};
use shacl_corpora::shacl12::{
    Body, Case12, InferCase, InferExpected, NodeExprCase, Root12, SrlCase, SrlKind, Status12,
    W3C12_CLAIMED_CLASSES, W3C12_SHACL_ROOT_APPROVED, W3C12_SUITE_COMMIT, W3C12_TOTAL_CASES,
    default_conformance_class, in_earl_scope, shacl12_cases,
};
use shacl_corpora::{Expected, Multiset, Tuple, W3cCase, file_iri, parse_turtle_file};

// ── Xfail ledger ──────────────────────────────────────────────────────────────

/// `rdf:reifies`.
use purrdf_iri::vocab::rdf::REIFIES as RDF_REIFIES;

/// Entries the engine currently fails, with the reason: `(test id, reason)`,
/// where the id is the entry's IRI relative to `vectors/shacl12/tests` (see
/// `shacl12::entry_id` for the SPARQL 1.2 RL evaluation entries). Grouped by
/// feature; each reason names what the engine does today, not a plan.
///
/// A ledgered entry MUST fail; when engine work fixes it the harness errors with
/// `XPASS` and the entry must be removed.
const XFAIL: &[(&str, &str)] = &[];

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

/// One row of the delta table: `(test id, the normative sentence quoted with its
/// section, the exact deltas)`.
type DeltaRow = (&'static str, &'static str, &'static [Amendment]);

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
/// The table is empty: the one vendored file whose expectation was amended here
/// is now listed by an upstream manifest and is graded as approved. The only
/// remaining unlisted entry is graded exactly as written.
const UNLISTED_FILE_DELTAS: &[DeltaRow] = &[];

/// [`UNLISTED_FILE_DELTAS`] pinned by count, so an entry cannot be added or
/// dropped without this number moving with it.
const UNLISTED_FILE_DELTAS_COUNT: usize = 0;

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
    graded_expectation_in(UNLISTED_FILE_DELTAS, id, listed, tc)
}

/// [`graded_expectation`] over an explicit delta `table`, so the refusal of a
/// listed entry is proven against a table that names one.
fn graded_expectation_in(
    table: &[DeltaRow],
    id: &str,
    listed: bool,
    tc: &W3cCase,
) -> Result<Expected, String> {
    let entry = table.iter().find(|(entry, ..)| *entry == id);
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

// ── sht:EvalNodeExpr ──────────────────────────────────────────────────────────

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
        None => eval_node_expr_in_scope(data, focus, expr, guard, scope)
            .map_err(|error| error.to_string()),
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
    // The node-expression loader and evaluator take no profile: node expressions exist
    // only in SHACL 1.2, so there is no compatibility alternative to select. The
    // report's law sentence (`shacl-ed-2026-10-16`) therefore covers the `sht:Validate`
    // and rule records, which load and run under that law, and not these records,
    // which no law selects.
    let (shapes, mut exprs) = shapes::from_dataset_with_node_expressions(
        &tc.dataset,
        &doc_prefixes,
        None,
        std::slice::from_ref(&tc.expr),
        &purrdf_shapes::ShapesImports::new(),
    )
    .map_err(|e| format!("shapes/node-expression parse error: {e}"))?;
    let expr = exprs
        .pop()
        .ok_or("the parser returned no expression for the one root it was given")?;

    let projected =
        engine::project_dataset(tc.dataset.as_ref()).map_err(|error| error.to_string())?;
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
    let mut guard = RecursionGuard::default();
    eval_in_scope(&data, &focus, &expr, &mut guard, &tc.scope, Scope::EMPTY)
}

/// Grade one `sht:EvalNodeExpr` entry.
fn run_node_expr(tc: &NodeExprCase) -> Result<(), String> {
    let produced = no_panic(|| eval_node_expr_case(tc))?;
    compare_outputs(&produced, &tc.expected, tc.ignore_order)
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
    // Rules run under the editor's draft law, which the suite's commit pins.
    let shapes = shapes::from_dataset_with_options(
        &shapes_dataset,
        None,
        &doc_prefixes,
        None,
        Some(tc.shapes_graph_iri.clone()),
        &purrdf_shapes::ShapesImports::new(),
        &engine::ValidationOptions::default().with_profile(ShaclProfile::ED_20261016),
    )
    .map_err(|e| format!("shapes parse error: {e}"))?;
    let data_dataset = if tc.data_path == tc.shapes_path {
        shapes_dataset
    } else {
        parse_turtle_file(&tc.data_path).map_err(|e| format!("data graph parse error: {e}"))?
    };
    let projected =
        engine::project_dataset(data_dataset.as_ref()).map_err(|error| error.to_string())?;
    let holder = ShaclData::new(Arc::clone(&projected), Arc::clone(&projected), None);
    let inferred = apply_rules(&holder, &shapes).map_err(|e| format!("apply_rules failed: {e}"))?;
    Ok((inferred, projected))
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
                    builder
                        .push_owned_reifier(&purrdf_rdf::RdfReifier::new(
                            s.to_rdf_term(),
                            purrdf_rdf::RdfTriple::new(
                                statement.subject.to_rdf_term(),
                                statement.predicate.as_str(),
                                statement.object.to_rdf_term(),
                            ),
                        ))
                        .unwrap();
                } else if reifiers.contains(&s) {
                    builder
                        .push_owned_annotation(&purrdf_rdf::RdfAnnotation::new(
                            s.to_rdf_term(),
                            predicate.as_str(),
                            o.to_rdf_term(),
                        ))
                        .unwrap();
                } else {
                    builder
                        .push_owned_quad(&RdfQuad::new(
                            s.to_rdf_term(),
                            predicate.as_str(),
                            o.to_rdf_term(),
                        ))
                        .unwrap();
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

/// Grade one SPARQL 1.2 RL entry through [`srl::check`] — the production check-only
/// entry point every host exposes (`purrdf rules --srl FILE --check`, Python
/// `check_rules`, WebAssembly `shaclCheckRules`, C `purrdf_shacl_check_rules`) — never
/// through harness-private stage calls.
///
/// A positive entry must be ACCEPTED at the [`srl::CheckLevel`] its type asks about: a
/// positive syntax test at [`srl::CheckLevel::Syntax`] (the suite: "regardless of
/// well-formedness and stratification" — sixteen of them are not stratifiable, and one is
/// not well formed), a positive well-formedness test at [`srl::CheckLevel::WellFormed`] (two
/// are not stratifiable), a positive stratification test at [`srl::CheckLevel::Stratified`].
/// A negative entry must be REFUSED at the full [`srl::CheckLevel::Stratified`] check, by
/// the stage its type names — a negative syntax test in the parser, a negative
/// well-formedness test in the §4.2 check of a document that parses, a negative
/// stratification test in the §4.4 stratifier of a well-formed rule set — so a test cannot
/// pass by failing for some other reason. The suite README: "All the test are
/// syntactically legal, i.e. conform to the SPARQL-RL Rules grammar" (well-formedness),
/// "All the test are syntactically legal and well-formed" (stratification).
///
/// An evaluation entry is checked by the same function, runs SPARQL 1.2 RL's infer
/// operation over `srlt:data`, and compares the INFERENCE graph — "The result of an
/// evaluation test is the inference graph" — with `mf:result` under RDF isomorphism
/// (RDFC-1.0 canonical N-Quads).
fn run_srl(tc: &SrlCase) -> Result<(), String> {
    let text = fs::read_to_string(&tc.ruleset)
        .map_err(|e| format!("cannot read {}: {e}", tc.ruleset.display()))?;
    let base = file_iri(&tc.ruleset);
    // SPARQL 1.2 RL exists only in the 1.2 family; `srl::check` and `srl::infer` take no
    // profile because there is no compatibility alternative to select.
    let check = |level| srl::check(&text, Some(&base), &[], level);
    let positive = |level| check(level).map(drop).map_err(|e| e.to_string());
    let expect_stage = |stage: &str| -> Result<(), String> {
        match &check(srl::CheckLevel::Stratified) {
            Err(error) => {
                let actual = srl_stage(error);
                if actual == stage {
                    Ok(())
                } else {
                    Err(format!(
                        "expected a {stage} failure, but the {actual} stage refused: {error}"
                    ))
                }
            }
            Ok(_) => Err(format!(
                "expected a {stage} failure, but every stage accepted it"
            )),
        }
    };
    match tc.kind {
        SrlKind::PositiveSyntax => positive(srl::CheckLevel::Syntax),
        SrlKind::PositiveWellFormedness => positive(srl::CheckLevel::WellFormed),
        SrlKind::PositiveStratification => positive(srl::CheckLevel::Stratified),
        SrlKind::NegativeSyntax => expect_stage("syntax"),
        SrlKind::NegativeWellFormedness => expect_stage("well-formedness"),
        SrlKind::NegativeStratification => expect_stage("stratification"),
        SrlKind::Eval => {
            let document = check(srl::CheckLevel::Stratified)
                .map_err(|e| e.to_string())?
                .into_document();
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
        SrlError::Import { .. } | SrlError::UnreachedImports { .. } => "import",
        _ => "evaluation",
    }
}

// ── The harness ───────────────────────────────────────────────────────────────

/// Grade one entry.
fn run(case: &Case12) -> Result<(), String> {
    match &case.body {
        Body::Validate(tc) => run_validate(&case.id, case.root.is_listed(), tc),
        Body::NodeExpr(tc) => run_node_expr(tc),
        Body::Infer(tc) => run_infer(tc),
        Body::Srl(tc) => run_srl(tc),
    }
}

// ── Grading, once ─────────────────────────────────────────────────────────────

/// Entries the engine fails against their UNAMENDED expectation because the
/// specification text contradicts that expectation: `(test id, the clause and the
/// observable difference)`. Such an entry is graded, reported `earl:failed` with
/// this rationale as its `earl:info`, and counted under its own scoreboard label. An entry that starts to
/// pass is refused (`XPASS`), as is an id no discovered entry has. Empty today:
/// every approved entry passes.
const SPEC_CONTRADICTED: &[(&str, &str)] = &[];

/// [`SPEC_CONTRADICTED`] pinned by count, so an entry cannot be added or dropped
/// without this number moving with it.
const SPEC_CONTRADICTED_COUNT: usize = 0;

/// This suite's report: its file, the full commit of the vendored suite, and the
/// profile each record carries.
const SUITE: w3c_earl::Suite = w3c_earl::Suite {
    file_name: "purrdf-shacl12-earl.ttl",
    commit: W3C12_SUITE_COMMIT,
    profile: "shacl-1.2",
    law: ShaclProfile::ED_20261016.id(),
};

/// The EARL-scope scoreboard line's label and the name its failed count carries.
const SCOPE_LABEL: &str = "W3C12 SHACL root";
const SCOPE_FAILED_LABEL: &str = "spec-contradicted";

/// One discovered entry and its grade against the unamended expectation.
struct Graded {
    id: String,
    section: String,
    type_label: String,
    root: Root12,
    status: Option<Status12>,
    /// The conformance classes the entry is reported under; empty outside the
    /// SHACL root.
    classes: Vec<String>,
    result: Result<(), String>,
}

/// Every discovered entry, graded exactly once per harness binary.
fn graded() -> &'static [Graded] {
    static GRADED: std::sync::OnceLock<Vec<Graded>> = std::sync::OnceLock::new();
    GRADED.get_or_init(|| {
        let cases = shacl12_cases();
        // Engine panics are caught by `no_panic` and graded as failures; the default
        // hook's backtraces would only drown the scoreboard.
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let graded = cases
            .iter()
            .map(|case| Graded {
                id: case.id.clone(),
                section: case.section.clone(),
                type_label: case.type_label(),
                root: case.root,
                status: case.status,
                classes: case.conformance_classes(),
                result: run(case),
            })
            .collect();
        std::panic::set_hook(default_hook);
        graded
    })
}

/// The entries a SHACL 1.2 EARL report covers: approved, reached from the SHACL
/// root.
fn scope_entries(graded: &[Graded]) -> Vec<w3c_earl::Entry<'_>> {
    let spec: BTreeMap<&str, &str> = SPEC_CONTRADICTED.iter().copied().collect();
    graded
        .iter()
        .filter(|g| in_earl_scope(g.root, g.status))
        .map(|g| w3c_earl::Entry {
            id: &g.id,
            result: &g.result,
            rationale: spec.get(g.id.as_str()).copied(),
        })
        .collect()
}

/// Per-section and per-type counts.
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    passed: usize,
    spec_contradicted: usize,
    xfailed: usize,
}

impl Tally {
    fn line(self) -> String {
        format!(
            "passed {:>3}  spec-contradicted {:>3}  xfailed {:>3}",
            self.passed, self.spec_contradicted, self.xfailed
        )
    }
}

/// The scoreboard's own tally, computed from the ledger match arms alone.
#[derive(Default)]
struct Scoreboard {
    errors: Vec<String>,
    sections: BTreeMap<String, Tally>,
    types: BTreeMap<String, Tally>,
    total: Tally,
    unlisted: usize,
    /// Each in-scope entry id, and whether it passed.
    scope: BTreeMap<String, bool>,
    /// Each claimed class: entries passed, entries in the class.
    classes: BTreeMap<String, (usize, usize)>,
}

fn scoreboard_of(graded: &[Graded]) -> Scoreboard {
    let xfail: BTreeMap<&str, &str> = XFAIL.iter().copied().collect();
    assert_eq!(
        xfail.len(),
        XFAIL.len(),
        "duplicate entries in the XFAIL ledger"
    );
    for (id, _) in XFAIL {
        assert!(
            graded.iter().any(|g| g.id == *id),
            "XFAIL ledger names unknown test {id} — stale entry?"
        );
    }
    w3c_earl::check_spec_table(SPEC_CONTRADICTED, SPEC_CONTRADICTED_COUNT, |id| {
        graded.iter().any(|g| g.id == id)
    })
    .unwrap_or_else(|error| panic!("{error}"));
    let spec: BTreeMap<&str, &str> = SPEC_CONTRADICTED.iter().copied().collect();

    let mut board = Scoreboard::default();
    for g in graded {
        if !g.root.is_listed() {
            // Graded and reported by `w3c_shacl12_unlisted_vendored_files`.
            board.unlisted += 1;
            continue;
        }
        let class = w3c_earl::classify(
            &g.id,
            &g.result,
            xfail.get(g.id.as_str()).copied(),
            spec.get(g.id.as_str()).copied(),
        );
        let class = match class {
            Ok(class) => class,
            Err(error) => {
                board.errors.push(error);
                continue;
            }
        };
        let bump = |tally: &mut Tally| match class {
            w3c_earl::Class::Passed => tally.passed += 1,
            w3c_earl::Class::SpecContradicted => tally.spec_contradicted += 1,
            w3c_earl::Class::Xfailed => tally.xfailed += 1,
        };
        bump(board.sections.entry(g.section.clone()).or_default());
        bump(board.types.entry(g.type_label.clone()).or_default());
        bump(&mut board.total);
        if in_earl_scope(g.root, g.status) {
            if class == w3c_earl::Class::Xfailed {
                // The EARL-scope line labels every failure spec-contradicted.
                board.errors.push(format!(
                    "{}: an XFAIL entry is in the EARL scope; the report would label it \
                     spec-contradicted",
                    g.id
                ));
            }
            let passed = class == w3c_earl::Class::Passed;
            board.scope.insert(g.id.clone(), passed);
            for class in &g.classes {
                let slot = board.classes.entry(class.clone()).or_default();
                slot.0 += usize::from(passed);
                slot.1 += 1;
            }
        }
    }
    board
}

fn scoreboard() -> Scoreboard {
    scoreboard_of(graded())
}

/// The EARL-scope lines, as printed: the SHACL-root line (a projection of the
/// report) and one line per claimed class.
fn scope_lines() -> Vec<String> {
    let board = scoreboard();
    let mut lines = vec![
        w3c_earl::report(&SUITE, w3c_earl::SAMPLE_COMMIT, &scope_entries(graded()))
            .scoreboard_line(SCOPE_LABEL, SCOPE_FAILED_LABEL),
    ];
    for class in W3C12_CLAIMED_CLASSES {
        let (passed, total) = board.classes.get(class).copied().unwrap_or_default();
        let name = class
            .strip_prefix(purrdf_iri::vocab::sht::NS)
            .expect("a claimed class is in the SHACL test namespace");
        lines.push(format!("W3C12 class {name}: passed {passed} of {total}"));
    }
    lines
}

#[test]
fn w3c_shacl12_conformance() {
    let graded = graded();
    assert_eq!(
        graded.len(),
        W3C12_TOTAL_CASES,
        "discovered SHACL 1.2 test count"
    );
    let board = scoreboard();
    let total = board.total;

    println!(
        "W3C SHACL 1.2 conformance scoreboard ({} approved tests; {} entries of \
         unlisted vendored files reported apart):",
        graded.len() - board.unlisted,
        board.unlisted
    );
    for (section, tally) in &board.sections {
        println!("  {section:<36} {}", tally.line());
    }
    println!("  by test type:");
    for (ty, tally) in &board.types {
        println!("  {ty:<36} {}", tally.line());
    }
    println!(
        "  W3C12 TOTAL: passed {}, spec-contradicted {}, xfailed {}, ledger {}",
        total.passed,
        total.spec_contradicted,
        total.xfailed,
        XFAIL.len()
    );
    for line in scope_lines() {
        println!("  {line}");
    }

    assert!(
        board.errors.is_empty(),
        "w3c_shacl12_conformance: {} error(s):\n{}",
        board.errors.len(),
        board.errors.join("\n\n")
    );
    assert_eq!(
        total.xfailed,
        XFAIL.len(),
        "xfail count must match the ledger exactly"
    );
    assert_eq!(
        total.spec_contradicted, SPEC_CONTRADICTED_COUNT,
        "spec-contradicted count must match the table exactly"
    );
    assert_eq!(
        board.unlisted, W3C12_UNLISTED_ENTRIES,
        "entries of unlisted vendored files"
    );
    assert_eq!(
        total.passed + total.spec_contradicted + total.xfailed,
        W3C12_TOTAL_CASES - W3C12_UNLISTED_ENTRIES,
        "every approved test must be a pass, a spec-contradicted entry or a ledgered xfail"
    );
}

// ── Unlisted vendored files ───────────────────────────────────────────────────

/// The number of entries in vendored files that no upstream manifest includes
/// (`shacl12::W3C12_UNINCLUDED_MANIFESTS`): `inference-rules/rdfs/rectangle-condition`.
const W3C12_UNLISTED_ENTRIES: usize = 1;

/// The entries of vendored files that no upstream manifest includes, graded
/// exactly against their own file — with the proven deltas in
/// [`UNLISTED_FILE_DELTAS`], none today, else as written — and reported
/// under their own label. They are not the approved suite and are never counted
/// among its passes. Every one must pass; there is no ledger here.
#[test]
fn w3c_shacl12_unlisted_vendored_files() {
    let cases = shacl12_cases();
    let unlisted: Vec<&Case12> = cases.iter().filter(|c| !c.root.is_listed()).collect();
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

/// The approved `sht:Validate` entries whose expected result rests on a sentence first
/// published after the working draft of 2026-09-18 (`shacl-wd-2026-09-18`, text
/// `32318223`) pass under the editor's-draft law (`shacl-ed-2026-10-16`) and are not met
/// by the working-draft law: it refuses the shapes graph at load or reports another
/// result, as its own text says. Their neighbours, the entries that rest on text the
/// working draft already has, pass under both laws, so the difference is the law's.
/// The rows name the editor's-draft sentences that the working-draft text
/// (`w3c/data-shapes` commit `32318223`) does not contain.
#[test]
fn the_entries_resting_on_post_wd_text_need_the_editors_draft_law() {
    const POST_WD: [&str; 6] = [
        "sparql/node/prefixes-003",
        "sparql/functions/blankNodeArgument-001",
        "inference-rules/rules-entailment-ruleset-validation",
        "inference-rules/rules-entailment-ruleset-includes",
        "inference-rules/rules-entailment-ruleset-multiple",
        "inference-rules/rules-entailment-ruleset-and-all",
    ];
    const WD_TEXT: [&str; 7] = [
        "sparql/node/deactivated-select-001",
        "sparql/node/sparql-deactivated-001",
        "sparql/functions/singleInstanceOf-001",
        "core/property/closed-001",
        "inference-rules/rules-entailment-validation",
        "sparql/pre-binding/pre-binding-009",
        "core/misc/deactivated-001",
    ];
    let mut seen = Vec::new();
    for case in shacl12_cases() {
        let post_wd = POST_WD.contains(&case.id.as_str());
        if !post_wd && !WD_TEXT.contains(&case.id.as_str()) {
            continue;
        }
        let Body::Validate(mut tc) = case.body else {
            panic!("{} is an sht:Validate test", case.id);
        };
        assert_eq!(tc.profile, ShaclProfile::ED_20261016, "{}", case.id);
        grade_against(&tc, &tc.expected)
            .unwrap_or_else(|error| panic!("{} fails under the editor's draft: {error}", case.id));
        tc.profile = ShaclProfile::WD_20260918;
        let under_the_working_draft = grade_against(&tc, &tc.expected);
        if post_wd {
            assert!(
                under_the_working_draft.is_err(),
                "{} rests on post-WD text, so the working-draft law must not meet it",
                case.id
            );
        } else {
            under_the_working_draft.unwrap_or_else(|error| {
                panic!(
                    "{} rests on working-draft text and must pass: {error}",
                    case.id
                )
            });
        }
        seen.push(case.id);
    }
    seen.sort();
    let mut expected: Vec<&str> = POST_WD.iter().chain(&WD_TEXT).copied().collect();
    expected.sort_unstable();
    assert_eq!(seen, expected, "every named entry is discovered");
}

/// No approved entry is graded against an amended expectation: a table row naming
/// a LISTED entry is refused, and the same entry with no row is graded against its
/// approved expectation.
#[test]
fn a_listed_entry_is_never_amended() {
    let cases = shacl12_cases();
    // Formerly the one amended entry; an upstream manifest now lists it.
    let id = "core/node/xone-003";
    let case = cases
        .iter()
        .find(|c| c.id == id)
        .expect("the entry is discovered");
    let Body::Validate(tc) = &case.body else {
        panic!("{id} is an sht:Validate test");
    };
    assert!(
        case.root.is_listed(),
        "{id} is an entry an upstream manifest lists"
    );
    const TUPLE: ResultTuple = ResultTuple {
        focus: ELSEWHERE,
        path: None,
        value: None,
        component: ELSEWHERE,
        severity: ELSEWHERE,
        source_shape: ELSEWHERE,
    };
    const CONTROL: &[Amendment] = &[Amendment {
        approved: TUPLE,
        graded: ResultTuple {
            path: Some(ELSEWHERE),
            ..TUPLE
        },
    }];
    let row: DeltaRow = (
        "core/node/xone-003",
        "SHACL 1.2 Core §6.7.2.2: \"control\"",
        CONTROL,
    );
    let error = graded_expectation_in(&[row], id, case.root.is_listed(), tc)
        .expect_err("a listed entry named in the table is refused");
    assert!(error.contains("an upstream manifest lists"), "{error}");
    assert!(
        graded_expectation_in(&[], id, case.root.is_listed(), tc).is_ok(),
        "the same entry with no row is graded against its approved expectation"
    );
}

// ── Roots, status and conformance classes ─────────────────────────────────────

/// The approved entries reachable from the SHACL root are exactly
/// [`W3C12_SHACL_ROOT_APPROVED`], every SHACL-root entry is approved, the SPARQL
/// 1.2 RL root holds exactly the `srlt:*` entries (which state no `mf:status`),
/// and the unlisted entries are the ones of `W3C12_UNINCLUDED_MANIFESTS`.
#[test]
fn the_shacl_root_scope_is_pinned() {
    let cases = shacl12_cases();
    let approved: Vec<&Case12> = cases
        .iter()
        .filter(|c| c.is_approved_shacl_root())
        .collect();
    assert_eq!(
        approved.len(),
        W3C12_SHACL_ROOT_APPROVED,
        "approved entries reachable from the SHACL root"
    );
    assert!(
        cases
            .iter()
            .filter(|c| c.root == Root12::Shacl)
            .all(|c| c.status == Some(Status12::Approved) && !matches!(c.body, Body::Srl(_))),
        "every SHACL-root entry is an approved, non-SPARQL-RL one"
    );
    for case in &cases {
        match case.root {
            Root12::SparqlRl => assert!(
                matches!(case.body, Body::Srl(_))
                    && case.status.is_none()
                    && case.requires.is_empty()
                    && case.id.starts_with("sparql-rl/"),
                "{}: a SPARQL 1.2 RL root entry is an srlt:* test with no status or class",
                case.id
            ),
            Root12::Shacl | Root12::Unlisted => assert!(
                !matches!(case.body, Body::Srl(_)) && case.status.is_some(),
                "{}: a SHACL-root or unlisted entry is an sht:* test with a status",
                case.id
            ),
        }
    }
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.status == Some(Status12::Proposed))
            .count(),
        0,
        "no vendored entry is proposed"
    );
    assert_eq!(
        cases.len() - approved.len(),
        W3C12_UNLISTED_ENTRIES + SRL_TOTAL,
        "everything outside the approved SHACL root is an unlisted entry or SPARQL 1.2 RL"
    );
}

/// The union of the conformance classes of every approved SHACL-root entry is
/// exactly the claimed set, no entry is left unclassified (the folder default
/// covers `core/`, `inference-rules/` and `node-expr/`; `sparql/` entries state
/// their class), and the default rule applies only when `mf:requires` is absent.
#[test]
fn the_claimed_classes_are_the_union_over_the_shacl_root() {
    let cases = shacl12_cases();
    let mut union: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for case in cases.iter().filter(|c| c.is_approved_shacl_root()) {
        let classes = case.conformance_classes();
        assert!(!classes.is_empty(), "{}: no conformance class", case.id);
        if case.requires.is_empty() {
            let folder = case.id.split('/').next().expect("an id has a folder");
            assert_eq!(
                classes,
                vec![
                    default_conformance_class(folder)
                        .expect("a default")
                        .to_owned()
                ],
                "{}: the folder default applies only without mf:requires",
                case.id
            );
        } else {
            assert_eq!(classes, case.requires, "{}: stated classes win", case.id);
        }
        union.extend(classes);
    }
    let claimed: std::collections::BTreeSet<String> = W3C12_CLAIMED_CLASSES
        .iter()
        .map(|c| (*c).to_owned())
        .collect();
    assert_eq!(
        claimed.len(),
        W3C12_CLAIMED_CLASSES.len(),
        "a class is claimed twice"
    );
    assert_eq!(
        union, claimed,
        "classes used by the SHACL root vs the claimed set"
    );

    // The folder defaults: the three the reports page knows, and no guess for
    // any other (a `sparql/` entry without `mf:requires` is unspecified).
    assert_eq!(
        default_conformance_class("core"),
        Some(purrdf_iri::vocab::sht::CONFORMANCE_CORE)
    );
    assert_eq!(
        default_conformance_class("inference-rules"),
        Some(purrdf_iri::vocab::sht::CONFORMANCE_INFERENCE_RULES)
    );
    assert_eq!(
        default_conformance_class("node-expr"),
        Some(purrdf_iri::vocab::sht::CONFORMANCE_NODE_EXPR)
    );
    assert_eq!(default_conformance_class("sparql"), None);
    assert_eq!(default_conformance_class("sparql-rl"), None);
}

/// The recorded suite commit is the one the vendored provenance names, in full.
#[test]
fn the_suite_commit_is_the_vendored_one() {
    assert_eq!(W3C12_SUITE_COMMIT.len(), 40);
    assert!(
        W3C12_SUITE_COMMIT
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    let provenance = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/shacl12/PROVENANCE.md"
    ))
    .expect("vectors/shacl12/PROVENANCE.md is vendored");
    assert!(
        provenance.contains(&format!("Pinned commit: `{W3C12_SUITE_COMMIT}`")),
        "PROVENANCE.md does not pin {W3C12_SUITE_COMMIT}"
    );
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
/// of an unlisted file) and nothing else. Passing here is a
/// grading verdict, not a count: how each entry is REPORTED is
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
const SRL_TOTAL: usize = 209;

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
            !case.root.is_listed(),
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

/// A computed `xsd:decimal` carries a point and a fractional digit: the value four
/// that `CEIL(3.2)` yields is spelled `"4.0"`, not `"4"`. Evaluated through the
/// same library path the harness grades with, on an `example.org` fixture.
#[test]
fn engine_spells_a_whole_computed_decimal_with_a_point() {
    let ttl = r"
        @prefix ex: <http://example.org/ns#> .
        @prefix sparql: <http://www.w3.org/ns/sparql#> .
        ex:root ex:expr [ sparql:ceil ( 3.2 ) ] .
    ";
    let dataset = purrdf_rdf::parse_dataset(ttl.as_bytes(), "text/turtle", None).expect("parse");
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
    let mut guard = RecursionGuard::default();
    let out = eval_node_expr_in_scope(
        &data,
        &Term::blank(ABSENT_FOCUS),
        &exprs[0],
        &mut guard,
        Scope::EMPTY,
    )
    .expect("evaluates");
    assert_eq!(out, [typed("4.0", "decimal")]);
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

/// The shared grader grades `sh:detail` exactly where an expected report states
/// it, and not where it states none.
///
/// No vendored report states `sh:detail` today, so the stated details are built
/// here over the approved `core/node/memberShape-001`: its `ex:list2` result gets
/// the one detail `"Bob"` and its `ex:list5` result exactly `"Charlie"` and
/// `"Donna"`, which are the details the engine really produces. The case passes
/// as approved with none stated and passes with them stated, and fails when the
/// stated details lose one, gain one, or change one field of one. Then the
/// control: with `ex:list5`'s stated details removed from the expectation, the
/// case still passes while the engine's own `ex:list5` result carries two details
/// — a produced detail the expectation does not spell is not graded (SHACL 1.2
/// Core §6.7.2.6, quoted in the grader's docs).
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
    assert!(
        tc.expected_details.is_empty(),
        "the approved report states no sh:detail"
    );
    shacl_corpora::report_grading::run_validate_case(tc)
        .expect("the case passes as approved with no details stated");

    let Expected::Report { results, .. } = &tc.expected else {
        panic!("memberShape-001 expects a report");
    };
    let list2 = "<http://example.com/ns#list2>";
    let list5 = "<http://example.com/ns#list5>";
    let parent = |focus: &str| -> Tuple {
        let mut found = results.keys().filter(|tuple| tuple.0 == focus);
        let tuple = found.next().expect("the approved report has the result");
        assert!(found.next().is_none(), "one approved result per focus node");
        tuple.clone()
    };
    let detail = |name: &str| ExpectedResult {
        tuple: (
            format!("\"{name}\""),
            None,
            Some(format!("\"{name}\"")),
            "<http://www.w3.org/ns/shacl#NodeKindConstraintComponent>".to_owned(),
            "<http://www.w3.org/ns/shacl#Violation>".to_owned(),
            "<http://example.com/ns#IRIShape>".to_owned(),
        ),
        details: None,
    };
    let stated = vec![
        ExpectedResult {
            tuple: parent(list2),
            details: Some(vec![detail("Bob")]),
        },
        ExpectedResult {
            tuple: parent(list5),
            details: Some(vec![detail("Charlie"), detail("Donna")]),
        },
    ];
    tc.expected_details = stated;
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
    let mut changed = stated5;
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

/// `validator-001` writes `owl:imports <http://datashapes.org/dash>` on a node — in the
/// SHACL 1.0 suite `<http://datashapes.org/sh/tests/sparql/component/validator-001.test>`,
/// in the SHACL 1.2 suite `<http://example.com/ns#>` — that is neither the document's own
/// IRI nor an `owl:Ontology` (nor names one as its `owl:versionIRI`). Under OWL 2's mapping to RDF
/// (§3.1.2) and SHACL 1.2 Core that triple is data, not an import. So in BOTH vendored
/// suites the case agrees with its approved report by name with an EMPTY import table,
/// and resolving its shapes graph under its own IRI reaches no document: the resolved
/// shapes graph is the parsed one, the same `Arc`, and the `owl:imports` triple is in it
/// as written.
///
/// The control reads the same file with that one node typed `owl:Ontology`. The triple
/// is then an import, and the same empty table refuses the graph naming exactly DASH —
/// so the pass is the anchor rule reading the triple as data, not a loader that ignores
/// `owl:imports`.
#[test]
fn validator_001_passes_in_both_suites_with_no_import_supplied() {
    const ID: &str = "sparql/component/validator-001";
    const DASH: &str = "http://datashapes.org/dash";
    use purrdf_iri::vocab::owl::IMPORTS as OWL_IMPORTS;

    let suite12 = shacl12_cases();
    let Body::Validate(tc12) = &suite12
        .iter()
        .find(|c| c.id == ID)
        .expect("validator-001 is discovered in the SHACL 1.2 suite")
        .body
    else {
        panic!("validator-001 is an sht:Validate test");
    };
    let suite10 = shacl_corpora::w3c_cases();
    let tc10 = suite10
        .iter()
        .find(|c| c.id == ID)
        .expect("validator-001 is discovered in the SHACL 1.0 suite");

    let empty = purrdf_shapes::ShapesImports::new();
    let iri = |value: &str| Term::NamedNode(NamedNode::new_unchecked(value));
    for (suite, tc, subject) in [
        (
            "SHACL 1.0",
            tc10,
            "http://datashapes.org/sh/tests/sparql/component/validator-001.test",
        ),
        ("SHACL 1.2", tc12, "http://example.com/ns#"),
    ] {
        shacl_corpora::report_grading::run_validate_case(tc).unwrap_or_else(|e| {
            panic!("{suite}: validator-001 agrees with its approved report: {e}")
        });

        let text = fs::read_to_string(&tc.shapes_path).expect("the case file reads");
        let own_iri = file_iri(&tc.shapes_path);
        let document = text_ingest::parse_turtle_document(&text, Some(&own_iri))
            .expect("the case file parses");
        let resolved = purrdf_shapes::resolve_shapes_imports(
            &document.dataset,
            &document.prefixes,
            &[own_iri.as_str()],
            &empty,
        )
        .unwrap_or_else(|e| panic!("{suite}: an empty table resolves the shapes graph: {e}"));
        assert!(
            Arc::ptr_eq(&resolved.dataset, &document.dataset),
            "{suite}: the closure reached no document, so the shapes graph is the parsed one"
        );
        assert_eq!(
            native_quads(
                resolved.dataset.as_ref(),
                Some(&iri(subject)),
                Some(&iri(OWL_IMPORTS)),
                Some(&iri(DASH)),
                GraphFilter::AnyGraph,
            )
            .len(),
            1,
            "{suite}: the owl:imports triple is in the shapes graph as data"
        );

        let anchored =
            format!("{text}\n<{subject}> a <http://www.w3.org/2002/07/owl#Ontology> .\n");
        let control = text_ingest::parse_turtle_document(&anchored, Some(&own_iri))
            .expect("the control parses");
        let Err(purrdf_shapes::ShapesError::Imports(
            purrdf_shapes::ShapesImportError::Unresolved { iris },
        )) = shapes::from_dataset_with_base(
            &control.dataset,
            Some(&own_iri),
            &control.prefixes,
            None,
            None,
            &empty,
        )
        else {
            panic!("{suite}: with its subject an owl:Ontology the triple is an import");
        };
        assert_eq!(
            iris,
            [DASH],
            "{suite}: the control is refused for exactly DASH"
        );
    }
}

// ── EARL ──────────────────────────────────────────────────────────────────────

/// The entries of `SPEC_CONTRADICTED` are graded against the unamended expectation
/// and fail it: the count is pinned, each id is a discovered approved SHACL-root
/// entry, and each is reported failed. An entry that passes, and an unknown id, are
/// refused beside the accepted neighbours.
#[test]
fn spec_contradicted_entries_fail_unamended() {
    let graded = graded();
    w3c_earl::check_spec_table(SPEC_CONTRADICTED, SPEC_CONTRADICTED_COUNT, |id| {
        graded.iter().any(|g| g.id == id)
    })
    .expect("the table is well formed");
    for (id, _) in SPEC_CONTRADICTED {
        let g = graded.iter().find(|g| g.id == *id).expect("discovered");
        assert!(
            in_earl_scope(g.root, g.status),
            "{id} is outside the EARL scope"
        );
        assert!(
            g.result.is_err(),
            "{id} passes: remove it from SPEC_CONTRADICTED"
        );
    }
    w3c_earl::check_classification();
}

/// A table row turns an entry's EARL outcome into `earl:failed` carrying the
/// rationale text only; a synthetic graded set exercises the path the empty table
/// does not.
#[test]
fn a_spec_contradicted_entry_is_reported_failed_with_its_rationale() {
    let failing = Err("engine said no, at /home/someone/tests/x.ttl".to_owned());
    let passing = Ok(());
    let rationale = "SHACL 1.2 Core, clause X: the approved result differs";
    let entries = [
        w3c_earl::Entry {
            id: "core/node/a-001",
            result: &passing,
            rationale: None,
        },
        w3c_earl::Entry {
            id: "core/node/b-001",
            result: &failing,
            rationale: Some(rationale),
        },
    ];
    let bytes = w3c_earl::earl_bytes(&w3c_earl::report(&SUITE, w3c_earl::SAMPLE_COMMIT, &entries))
        .expect("renders");
    let written = w3c_earl::assertions_by_key(&bytes).expect("parses");
    assert_eq!(
        written["core/node/a-001"].outcome,
        purrdf_iri::vocab::earl::PASSED
    );
    let failed = &written["core/node/b-001"];
    assert_eq!(failed.outcome, purrdf_iri::vocab::earl::FAILED);
    assert_eq!(
        failed.info.as_deref(),
        Some(w3c_earl::literal(rationale).as_str())
    );
    let text = String::from_utf8(bytes).expect("UTF-8");
    assert!(
        !text.contains("/home/someone"),
        "no machine path reaches the report"
    );
    assert_eq!(
        text.matches("earl#info").count(),
        1,
        "only the failed entry has info"
    );
}

/// The mapping, the environment parsing, the key port and the write path.
#[test]
fn w3c12_earl_support_behaviour() {
    w3c_earl::check_record_mapping(&SUITE);
    w3c_earl::check_config_parsing();
    w3c_earl::check_test_key();
    w3c_earl::check_write_errors();
}

/// Exactly the approved SHACL-root entries are assertions, once each, every one
/// `earl:automatic`, and their keys are the keys the Working Group's reports page
/// derives for the same entries: the page's `testKey` ported, applied to each entry
/// IRI, and the page's class rule giving every entry a claimed class.
#[test]
fn w3c12_earl_keys_equal_wg_page_keys() {
    let bytes = w3c_earl::sample_bytes(&SUITE, &scope_entries(graded()));
    let written = w3c_earl::assertions_by_key(&bytes).expect("the report parses");
    assert_eq!(
        written.len(),
        W3C12_SHACL_ROOT_APPROVED,
        "one assertion per entry"
    );
    assert!(
        written
            .values()
            .all(|a| a.mode == purrdf_iri::vocab::earl::AUTOMATIC),
        "every assertion is earl:automatic"
    );
    let tests = format!("{}/", file_iri(&shacl_corpora::shacl12::shacl12_root()));
    let page_keys: std::collections::BTreeSet<String> = graded()
        .iter()
        .filter(|g| in_earl_scope(g.root, g.status))
        .map(|g| w3c_earl::test_key(&format!("{tests}{}", g.id), &tests))
        .collect();
    assert_eq!(
        page_keys.len(),
        W3C12_SHACL_ROOT_APPROVED,
        "keys are distinct"
    );
    let earl_keys: std::collections::BTreeSet<String> = written.keys().cloned().collect();
    assert_eq!(earl_keys, page_keys, "EARL keys vs the page's keys");
    // The page's class rule: every entry in scope has a claimed class.
    for g in graded().iter().filter(|g| in_earl_scope(g.root, g.status)) {
        assert!(
            !g.classes.is_empty()
                && g.classes
                    .iter()
                    .all(|c| W3C12_CLAIMED_CLASSES.contains(&c.as_str())),
            "{}: classes {:?} are not all claimed",
            g.id,
            g.classes
        );
    }
}

/// Each entry's EARL outcome is its unamended grade, read directly from the grade
/// and not from the scoreboard: passed exactly when the harness passed it, failed
/// otherwise, and nothing but those two outcomes.
#[test]
fn w3c12_earl_outcomes_equal_the_unamended_grades() {
    let written =
        w3c_earl::assertions_by_key(&w3c_earl::sample_bytes(&SUITE, &scope_entries(graded())))
            .expect("parses");
    for g in graded().iter().filter(|g| in_earl_scope(g.root, g.status)) {
        let outcome = &written[&g.id].outcome;
        assert_eq!(
            outcome,
            if g.result.is_ok() {
                purrdf_iri::vocab::earl::PASSED
            } else {
                purrdf_iri::vocab::earl::FAILED
            },
            "{}",
            g.id
        );
        assert_eq!(
            written[&g.id].info.is_some(),
            SPEC_CONTRADICTED.iter().any(|(id, _)| *id == g.id),
            "{}: earl:info exactly for SPEC_CONTRADICTED entries",
            g.id
        );
    }
    assert!(written.values().all(|a| {
        a.outcome == purrdf_iri::vocab::earl::PASSED || a.outcome == purrdf_iri::vocab::earl::FAILED
    }));
}

/// The subject and the assertor, by exact value, with the sample commit and this
/// suite's full commit.
#[test]
fn w3c12_earl_subject_and_assertor_are_exact() {
    w3c_earl::check_descriptions(
        &w3c_earl::sample_bytes(&SUITE, &scope_entries(graded())),
        w3c_earl::SAMPLE_COMMIT,
        "4cd469867a1cc6d2fff80e8a40723b26c695d67a",
        "shacl-ed-2026-10-16",
    )
    .unwrap_or_else(|error| panic!("{error}"));
}

/// Two in-memory generations are byte-identical, and no date or time predicate (or
/// datatype) appears anywhere in the report.
#[test]
fn w3c12_earl_is_deterministic_and_carries_no_date() {
    let first = w3c_earl::sample_bytes(&SUITE, &scope_entries(graded()));
    assert_eq!(
        first,
        w3c_earl::sample_bytes(&SUITE, &scope_entries(graded()))
    );
    let text = String::from_utf8(first.clone()).expect("UTF-8");
    assert!(!text.contains("dateTime") && !text.contains("XMLSchema#date"));
    for predicate in w3c_earl::predicates(&first).expect("parses") {
        assert!(
            !predicate.to_ascii_lowercase().contains("date")
                && !predicate.to_ascii_lowercase().contains("time"),
            "{predicate} is a date or time predicate"
        );
    }
    let mut sorted: Vec<&str> = text.lines().collect();
    let written = sorted.clone();
    sorted.sort_unstable();
    assert_eq!(written, sorted, "the file is sorted");
}

/// A proposed entry is never an assertion: a synthetic graded set holding a
/// proposed SHACL-root entry, an approved one, and an entry of another root.
#[test]
fn w3c12_proposed_entries_are_excluded_from_earl() {
    let synthetic = |id: &str, root, status| Graded {
        id: id.to_owned(),
        section: "core/node".to_owned(),
        type_label: "sht:Validate".to_owned(),
        root,
        status,
        classes: Vec::new(),
        result: Ok(()),
    };
    let set = [
        synthetic(
            "core/node/approved-001",
            Root12::Shacl,
            Some(Status12::Approved),
        ),
        synthetic(
            "core/node/proposed-001",
            Root12::Shacl,
            Some(Status12::Proposed),
        ),
        synthetic("sparql-rl/eval/other-001", Root12::SparqlRl, None),
        synthetic(
            "core/node/unlisted-001",
            Root12::Unlisted,
            Some(Status12::Approved),
        ),
    ];
    let bytes = w3c_earl::sample_bytes(&SUITE, &scope_entries(&set));
    let written = w3c_earl::assertions_by_key(&bytes).expect("parses");
    assert_eq!(
        written.keys().collect::<Vec<_>>(),
        ["core/node/approved-001"]
    );
}

/// The EARL and the scoreboard agree, checked independently of the shared report:
/// the printed SHACL-root line and the serialized report are compared, per entry
/// and in total, with the tally the match arms computed before any report existed;
/// and each of three mutations of the serialized report alone breaks the agreement.
#[test]
fn earl_agrees_with_scoreboard() {
    let printed = scope_lines().remove(0);
    let board = scoreboard();
    assert_eq!(board.scope.len(), W3C12_SHACL_ROOT_APPROVED);
    w3c_earl::check_agreement_and_mutations(
        &w3c_earl::sample_bytes(&SUITE, &scope_entries(graded())),
        &printed,
        SCOPE_LABEL,
        SCOPE_FAILED_LABEL,
        &board.scope,
    );
}

/// The per-class lines add up: every in-scope entry is counted under each of its
/// classes, and the claimed classes are all the classes there are.
#[test]
fn w3c12_class_lines_cover_the_scope() {
    let board = scoreboard();
    let lines = scope_lines();
    assert_eq!(lines.len(), 1 + W3C12_CLAIMED_CLASSES.len());
    assert_eq!(board.classes.keys().cloned().collect::<Vec<_>>(), {
        let mut claimed: Vec<String> = W3C12_CLAIMED_CLASSES
            .iter()
            .map(|c| (*c).to_owned())
            .collect();
        claimed.sort();
        claimed
    });
    let memberships: usize = graded()
        .iter()
        .filter(|g| in_earl_scope(g.root, g.status))
        .map(|g| g.classes.len())
        .sum();
    assert_eq!(
        board
            .classes
            .values()
            .map(|(_, total)| total)
            .sum::<usize>(),
        memberships
    );
    for line in &lines[1..] {
        assert!(
            line.starts_with("W3C12 class ") && line.contains(": passed "),
            "{line}"
        );
    }
}

/// With both settings the report is written under the directory; with neither,
/// nothing is; with either alone or a malformed commit, the run fails.
#[test]
fn w3c12_earl_report_is_written_when_configured() {
    let config = match w3c_earl::config() {
        Err(error) => panic!("{error}"),
        Ok(None) => {
            println!("W3C12 EARL: no {} set; nothing written", w3c_earl::DIR_VAR);
            return;
        }
        Ok(Some(config)) => config,
    };
    let bytes = w3c_earl::earl_bytes(&w3c_earl::report(
        &SUITE,
        &config.commit,
        &scope_entries(graded()),
    ))
    .expect("the report renders");
    w3c_earl::check_descriptions(&bytes, &config.commit, W3C12_SUITE_COMMIT, SUITE.law)
        .unwrap_or_else(|error| panic!("{error}"));
    let path = w3c_earl::write_report(&config.dir, SUITE.file_name, &bytes)
        .unwrap_or_else(|error| panic!("{error}"));
    println!(
        "W3C12 EARL: wrote {} ({} bytes)",
        path.display(),
        bytes.len()
    );
}
