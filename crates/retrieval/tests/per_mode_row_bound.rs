// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The declared row bound is a function of the MODE, and the layer reads it at the
//! mode it actually invokes.
//!
//! `PropertyFunction::rows_per_invocation` takes a
//! [`BindingPattern`](purrdf_sparql_eval::BindingPattern) and is a real function of it:
//! an index-backed producer declares many modes precisely because its indices serve
//! binding directions a scan cannot, and it can honestly promise three rows where its
//! needle is bound and a hundred where it is free. The published producer contract
//! encourages exactly that, and promises in return that the waist refuses a recorded
//! depth above the declaration and that the number a producer is handed is never raised
//! past its own.
//!
//! Both promises were false for exactly that configuration. The planner and the waist
//! each read the declaration at **every** declared mode and took the maximum, which is
//! a figure no invocation is ever made under, so a producer's *other* mode decided its
//! depth, its refusals and the number it was asked for. Nothing in the suite could see
//! it: every other fixture declares one mode and ignores the parameter, so the per-mode
//! axis was unreachable.
//!
//! This file is that axis. [`ModedProducer`] declares two modes with two different row
//! counts, and every case below is executed beside the single-mode control that
//! declares only the invoked mode's number — because a layer that read the *wrong*
//! smaller number would pass a test that only ever asserted the smaller one. The
//! controls are what say the fix is a correct reading rather than a tighter one.
//!
//! Fixtures are `example.org` throughout.

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::TermValue;
use purrdf_retrieval::{
    AdmissionEnvironment, AdmissionError, BoundMode, ExecutionError, Iri, ProducerStatus,
    RequestTerm, RetrievalRequest, Statistics, compile, execute, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, CandidateDomains, DepthPlacement, DuplicatePolicy, EvalError,
    PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, RankFidelity,
    RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
};

mod common;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

const DOCS: &str = "stratum/docs";
const PRODUCER: &str = "pf/docs";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The needle this file plans with. It carries a predicate, so the producer's
/// declaration can accept it by shape and place it into an argument position.
fn needle() -> RequestTerm {
    RequestTerm::Lexical {
        text: "quick brown fox".to_owned(),
        language: None,
        predicate: Some(iri(&ex("body"))),
    }
}

/// A producer whose declared row bound is a **real function of the mode**.
///
/// The suite had no such fixture. Every other mock writes
/// `fn rows_per_invocation(&self, _mode: BindingPattern)` and answers one number, so the
/// maximum over the declared modes and the number at the invoked mode were always equal
/// and the difference between them was untestable.
///
/// This one answers `bound_rows` where the needle position is bound and `free_rows`
/// where it is not, which is the shape every index-backed producer in the tree has: a
/// bound needle is an index lookup with few rows behind it, a free needle is a scan of
/// the whole partition.
struct ModedProducer {
    arity: PfArity,
    /// Every access mode the relation declares, in declaration order.
    modes: Vec<BindingPattern>,
    /// The bound declared where the needle position is bound.
    bound_rows: u64,
    /// The bound declared where it is free.
    free_rows: u64,
    /// The rows it actually returns, in rank order.
    emitted: Vec<Vec<TermValue>>,
    /// The position its needle is placed at, which is the position the declared
    /// bound varies over.
    needle_position: usize,
    /// Where this producer takes its own row request, for the self-bounding case.
    ///
    /// `Some(position)` makes the relation refuse a request for more rows than the
    /// invoked mode declares — the guard the shipped nearest-neighbour relation has,
    /// and the one that turns "the layer asked for the wrong number" from a quiet
    /// over-read into a lost stratum.
    depth_position: Option<usize>,
}

impl ModedProducer {
    /// The bound this relation declares for `mode`, spelled once so the declaration
    /// and the guard below cannot disagree about it.
    fn declared_for(&self, mode: BindingPattern) -> u64 {
        if mode.is_bound(self.needle_position) {
            self.bound_rows
        } else {
            self.free_rows
        }
    }
}

impl PropertyFunction for ModedProducer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        self.arity
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        self.declared_for(mode)
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        // How many rows this call may return. A self-bounding relation is bounded by
        // the request in its own argument and by nothing else; one the evaluator bounds
        // takes no request and is cut by the branch's `LIMIT`.
        let asked = match self.depth_position {
            None => usize::MAX,
            Some(position) => {
                let requested = args
                    .get(position)
                    .and_then(|value| match value {
                        TermValue::Literal { lexical_form, .. } => lexical_form.parse::<u64>().ok(),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        EvalError::function(format!(
                            "the row request at position {position} is free or not an integer; \
                             this relation bounds itself and cannot be called without one"
                        ))
                    })?;
                let declared = self.declared_for(args.mode());
                if requested > declared {
                    // The refusal the whole per-mode reading exists to avoid asking
                    // for. A producer handed a number above its own registration cannot
                    // answer it without contradicting the registry, and answering fewer
                    // rows as if they were the whole read is the silent narrowing it
                    // must not do.
                    return Err(EvalError::function(format!(
                        "the relation declares {declared} row(s) in mode {} and was asked for \
                         {requested}",
                        args.mode().code()
                    )));
                }
                usize::try_from(requested).unwrap_or(usize::MAX)
            }
        };
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        let mut rows = Vec::with_capacity(self.emitted.len());
        for row in self.emitted.iter().take(asked) {
            let mut echoed = Vec::with_capacity(row.len());
            for (position, value) in row.iter().enumerate() {
                echoed.push(
                    bound
                        .get(position)
                        .and_then(Clone::clone)
                        .unwrap_or_else(|| value.clone()),
                );
            }
            rows.push(echoed);
        }
        Ok(Box::new(RowCursor {
            rows: rows.into_iter(),
        }))
    }
}

struct RowCursor {
    rows: std::vec::IntoIter<Vec<TermValue>>,
}

impl PfCursor for RowCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.rows.next())
    }
}

/// How the producer under test bounds its own read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// The evaluator bounds the branch with a `LIMIT`: arity `(1, 1)`, candidate at 0
    /// and needle at 1.
    EvaluatorBounded,
    /// The producer takes the row request as an argument and refuses one above its own
    /// declaration: arity `(1, 2)`, candidate at 0, needle at 1, request at 2.
    SelfBounding,
}

/// How many modes the producer declares.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Modes {
    /// Two: the invoked one, and a second, larger one the layer must not read.
    ///
    /// This is the configuration the producer contract encourages, and the one every
    /// defect below was reachable through.
    Both,
    /// One: exactly the invoked mode. The CONTROL — the number the two-mode case must
    /// agree with, so a fix is a correct reading rather than a tighter one.
    InvokedOnly,
}

/// A registry holding one producer of `shape`, declaring `modes`, promising
/// `bound_rows` where its needle is bound and `free_rows` where it is not, and holding
/// `held` rows.
fn registry(
    shape: Shape,
    modes: Modes,
    bound_rows: u64,
    free_rows: u64,
    held: usize,
) -> PropertyFunctionRegistry {
    let arity = match shape {
        Shape::EvaluatorBounded => PfArity::new(1, 1),
        Shape::SelfBounding => PfArity::new(1, 2),
    };
    let total = arity.total();
    // The invoked mode: the needle at position 1 is bound by the planned binding,
    // and the row request at position 2 — where there is one — by the layer.
    let invoked = BindingPattern::from_bools(
        (0..total).map(|position| position == 1 || (shape == Shape::SelfBounding && position == 2)),
    );
    // The second declared mode leaves the needle free and so declares `free_rows`. It
    // subsumes the invoked mode — it demands strictly less — which is exactly why
    // reading the declaration at "every mode" reached it at all.
    let needle_free = BindingPattern::from_bools(
        (0..total).map(|position| shape == Shape::SelfBounding && position == 2),
    );
    let declared = match modes {
        Modes::Both => vec![invoked, needle_free],
        Modes::InvokedOnly => vec![invoked],
    };
    let emitted = (0..held)
        .map(|index| {
            (0..total)
                .map(|position| TermValue::iri(format!("{}{position}/{index}", ex("row/"))))
                .collect()
        })
        .collect();
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex(PRODUCER),
        Arc::new(ModedProducer {
            arity,
            modes: declared,
            bound_rows,
            free_rows,
            emitted,
            needle_position: 1,
            depth_position: match shape {
                Shape::EvaluatorBounded => None,
                Shape::SelfBounding => Some(2),
            },
        }),
        RankedDeclaration {
            stratum: purrdf_core::parse_iri(&ex(DOCS)).expect("fixture IRI"),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern::of_kind(TermKind::Literal),
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }],
            }],
            depth_placement: match shape {
                Shape::EvaluatorBounded => None,
                Shape::SelfBounding => Some(DepthPlacement {
                    position: 2,
                    datatype: XSD_INTEGER.to_owned(),
                }),
            },
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            // This mock answers from a list it holds entire: its search names
            // every row that was due to it and ranks them by the order it was
            // given, so the exhaustive declaration is the true one. It is
            // stated rather than left out, because there is no default.
            fidelity: RankFidelity::EXACT,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            mandatory: false,
        },
    );
    registry
}

/// A statistics provider that reports nothing, so every depth below is the
/// registry's declaration and never a measurement narrowing it.
///
/// The label and the revision are owned fields rather than literals, because the trait
/// hands both back by reference and a provider's identity is the caller's data.
struct NoStatistics {
    source: String,
    revision: String,
}

impl NoStatistics {
    /// The provider these tests plan against.
    fn new() -> Self {
        Self {
            source: "example.org/statistics/none".to_owned(),
            revision: "r1".to_owned(),
        }
    }
}

impl Statistics for NoStatistics {
    fn source(&self) -> &str {
        &self.source
    }

    fn revision(&self) -> &str {
        &self.revision
    }

    fn cardinality(&self, _subject: &Iri) -> Option<u64> {
        None
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

/// The depth and the declared row bound the layer records for one registry.
fn planned(registry: &PropertyFunctionRegistry) -> (u32, Option<u64>) {
    let stats = NoStatistics::new();
    let request = RetrievalRequest::complete(vec![needle()]);
    let planned = plan(&request, registry, &stats).expect("the needle plans");
    let env = AdmissionEnvironment {
        registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("the plan is admitted");
    let unit = compiled
        .units
        .iter()
        .find(|unit| unit.stratum == iri(&ex(DOCS)))
        .expect("the one stratum emits");
    (unit.depth(), unit.declared_rows())
}

/// Run one registry's plan end to end and report the stratum's status.
fn status(registry: &PropertyFunctionRegistry) -> ProducerStatus {
    run(registry).expect("the units run").1
}

/// Plan, compile and execute one registry, returning the compiled depth beside the
/// stratum's status.
fn run(registry: &PropertyFunctionRegistry) -> Result<(u32, ProducerStatus), ExecutionError> {
    let stats = NoStatistics::new();
    let request = RetrievalRequest::complete(vec![needle()]);
    let planned = plan(&request, registry, &stats).expect("the needle plans");
    let env = AdmissionEnvironment {
        registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled = compile(&planned, &env).expect("the plan is admitted");
    let depth = compiled.units[0].depth();
    let execution = block_on(execute(&compiled, registry, &*common::empty_dataset()))?;
    let status = execution.statuses[&iri(&ex(DOCS))].clone();
    Ok((depth, status))
}

/// Raise `stratum/docs`'s depth to `depth` in a fresh plan and admit it.
fn admit_at_depth(registry: &PropertyFunctionRegistry, depth: u32) -> Result<(), AdmissionError> {
    let stats = NoStatistics::new();
    let request = RetrievalRequest::complete(vec![needle()]);
    let mut planned = plan(&request, registry, &stats).expect("the needle plans");
    planned.stratum_depths.insert(iri(&ex(DOCS)), depth);
    let env = AdmissionEnvironment {
        registry,
        statistics: &stats,
        fusion_profile: None,
    };
    compile(&planned, &env).map(|_| ())
}

// ---------------------------------------------------------------------------
// The fixture is a real function of its mode
// ---------------------------------------------------------------------------

/// The premise every case below rests on, asserted rather than assumed: this fixture's
/// declaration really does differ between its two modes.
///
/// Without this, a layer that read the declaration at the wrong mode and a layer that
/// read it at the right one would be indistinguishable here — which is precisely the
/// state the rest of the suite was in.
#[test]
fn the_fixture_declares_a_different_bound_under_each_mode() {
    let producer = ModedProducer {
        arity: PfArity::new(1, 1),
        modes: vec![
            BindingPattern::from_code("fb"),
            BindingPattern::from_code("ff"),
        ],
        bound_rows: 3,
        free_rows: 100,
        emitted: Vec::new(),
        needle_position: 1,
        depth_position: None,
    };
    assert_eq!(
        producer.rows_per_invocation(BindingPattern::from_code("fb")),
        3,
        "the needle bound is an index lookup: three rows"
    );
    assert_eq!(
        producer.rows_per_invocation(BindingPattern::from_code("ff")),
        100,
        "the needle free is a scan: a hundred"
    );
}

// ---------------------------------------------------------------------------
// A. The depth is the invoked mode's, and a depth above it is refused
// ---------------------------------------------------------------------------

/// A depth is planned at — and held to — the bound the INVOKED mode declares, not the
/// largest bound the producer declares anywhere.
///
/// Read at the maximum over the declared modes, this producer's depth came out at its
/// *other* mode's hundred: the plan recorded a depth of nine over an invocation whose
/// mode declares three, `DepthBoundViolation` never fired at any depth up to a hundred,
/// and the read then reported `Exhausted { rows_emitted: 3 }` — a completeness claim
/// for a depth the producer had said it could not serve.
///
/// Both halves are executed. The two-mode producer must plan at three and refuse four;
/// the single-mode control, declaring only the invoked mode's three, must plan at the
/// same three and refuse the same four — because a depth read at the right mode and a
/// depth narrowed by accident are the same number only when the control agrees.
#[test]
fn a_depth_is_planned_and_admitted_at_the_invoked_modes_declaration() {
    let two_modes = registry(Shape::EvaluatorBounded, Modes::Both, 3, 100, 20);
    assert_eq!(
        planned(&two_modes),
        (3, Some(3)),
        "the invoked mode declares three rows, so three is the depth and three is the \
         bound the unit carries — the hundred belongs to a mode this call is not made \
         under"
    );

    // The control: the same producer declaring only the mode it is invoked under.
    let one_mode = registry(Shape::EvaluatorBounded, Modes::InvokedOnly, 3, 100, 20);
    assert_eq!(
        planned(&one_mode),
        (3, Some(3)),
        "and a producer that declares nothing else agrees, which is what makes the \
         number above a reading of the declaration rather than a coincidence"
    );

    // A depth above the invoked mode's declaration is refused, at the dimension that
    // names the registry. Nine was the depth the maximum used to admit here.
    for registry in [&two_modes, &one_mode] {
        let error = admit_at_depth(registry, 9).expect_err("a depth above the declaration");
        match error {
            AdmissionError::DepthBoundViolation {
                stratum,
                declared,
                requested,
                mode,
            } => {
                assert_eq!(*stratum, iri(&ex(DOCS)));
                assert_eq!(
                    declared, 3,
                    "the number refused against is the invoked mode's"
                );
                assert_eq!(requested, 9);
                assert_eq!(
                    mode,
                    BoundMode::Invoked {
                        mode: BindingPattern::from_code("fb")
                    },
                    "and the refusal names the mode it was read at — here the invoked \
                     one, because no coarser mode declares less"
                );
            }
            other => panic!("expected DepthBoundViolation, got {other:?}"),
        }
    }

    // The neighbouring depths that must still admit, so the refusal above is a bound
    // and not a lock: every depth from one up to the declaration itself.
    for depth in 1..=3 {
        for registry in [&two_modes, &one_mode] {
            admit_at_depth(registry, depth)
                .unwrap_or_else(|error| panic!("depth {depth} is inside the declaration: {error}"));
        }
    }
    // And the first depth past it is refused for both, which is where the bound is.
    for registry in [&two_modes, &one_mode] {
        admit_at_depth(registry, 4).expect_err("a depth one past the declaration");
    }
}

// ---------------------------------------------------------------------------
// B. A row-bound breach is not defeated by declaring a second, larger mode
// ---------------------------------------------------------------------------

/// A producer that beats the bound its INVOKED mode declared is refused, whether or
/// not it also declares a larger mode.
///
/// Declaring a second, larger mode used to defeat `RowBoundBreached` outright: the same
/// producer, the same twelve rows behind it, the same declaration of three under the
/// mode it was called in — refused when it declared only that mode, and served as an
/// ordinary `DepthReached` when it also declared a mode promising a hundred.
///
/// The valid neighbour is executed too: a producer holding exactly what its invoked
/// mode declared is served and certified, so the refusal is about the breach and not
/// about the second mode.
#[test]
fn a_breach_of_the_invoked_modes_bound_is_refused_with_or_without_a_second_mode() {
    for modes in [Modes::Both, Modes::InvokedOnly] {
        let over = registry(Shape::EvaluatorBounded, modes, 3, 100, 12);
        let error = run(&over).expect_err("twelve rows behind a declaration of three");
        match &error {
            ExecutionError::RowBoundBreached {
                stratum,
                declared,
                pulled,
                mode,
            } => {
                assert_eq!(**stratum, iri(&ex(DOCS)));
                assert_eq!(
                    *declared, 3,
                    "the promise the producer broke is its invoked mode's"
                );
                assert_eq!(
                    *pulled, 4,
                    "the read reaches one row past the depth, which is the probe that \
                     makes the breach observable"
                );
                assert_eq!(
                    *mode,
                    BoundMode::Invoked {
                        mode: BindingPattern::from_code("fb")
                    },
                    "and the mode the broken promise was read at is the invoked one, \
                     since the needle-free mode declares more rather than less"
                );
                assert!(
                    error.to_string().contains(
                        "declares at most 3 rows per invocation under mode `fb`, and the \
                         read returned 4"
                    ),
                    "which is what a host reading only the message is told: {error}"
                );
            }
            other => panic!("expected RowBoundBreached, got {other:?}"),
        }

        // The neighbour that must still be served: the producer holds exactly what it
        // declared, the probe row finds nothing behind it, and the read is certified.
        let honest = registry(Shape::EvaluatorBounded, modes, 3, 100, 3);
        assert_eq!(
            run(&honest).expect("an honest producer runs"),
            (3, ProducerStatus::Exhausted { rows_emitted: 3 }),
            "three rows behind a declaration of three is an exhaustion this layer can \
             verify"
        );

        // And one row short of it, so the certificate is not an artefact of the count
        // meeting the depth.
        let short = registry(Shape::EvaluatorBounded, modes, 3, 100, 2);
        assert_eq!(
            status(&short),
            ProducerStatus::Exhausted { rows_emitted: 2 },
            "and a producer with less than it promised is complete, not breached"
        );
    }
}

// ---------------------------------------------------------------------------
// B2. A coarser mode declaring LESS is the bound, and the refusal names that mode
// ---------------------------------------------------------------------------

/// Where a **coarser** declared mode promises fewer rows than the invoked one, the
/// coarser number is the bound — and the refusal it produces names the mode it was read
/// at, because that mode is not the one the call was made under.
///
/// Every case above is monotone in the mode: `fb` declares three and `ff` a hundred, so
/// the tightest promise among the modes that serve an `fb` call and the invoked mode's
/// own number are the same three, and nothing here could tell a layer that takes the
/// minimum from one that simply reads the invoked mode. This inverts the fixture — `fb`
/// declares a hundred and `ff` three — which separates them.
///
/// The minimum is the sound reading and not a tie-break between two opinions. `ff`
/// demands strictly fewer bindings than `fb`, so every tuple an `fb` call can emit is
/// one an `ff` call can emit too — the extra binding only filters — and a producer
/// promising three rows with its needle free cannot honestly promise a hundred with it
/// bound. Such a producer has over-declared `fb`, the three bounds the read, and a
/// producer that then returns four has broken a promise it really did register.
///
/// What the refusal has to say is *which* promise. Named as "at most 3 rows per
/// invocation" alone, with no mode anywhere, the figure appears nowhere in the mode the
/// call was made at, and its author goes looking at the `fb` declaration that says a
/// hundred.
///
/// The control is the same producer with the coarser mode removed: `fb` alone declaring
/// a hundred, where the twenty rows it holds are served and nothing is refused. That is
/// what makes the refusal above a reading of the `ff` declaration rather than a tighter
/// reading of anything else.
#[test]
fn a_coarser_mode_declaring_less_is_the_bound_and_the_refusal_names_that_mode() {
    let inverted = registry(Shape::EvaluatorBounded, Modes::Both, 100, 3, 20);
    assert_eq!(
        planned(&inverted),
        (3, Some(3)),
        "the needle-free mode declares three and subsumes this call, so three is the \
         depth and three is the bound — the hundred the invoked mode declares is an \
         over-declaration of a read its own coarser mode already bounds"
    );

    // The breach, with twenty rows behind a bound of three.
    let error = run(&inverted).expect_err("twenty rows behind a declaration of three");
    match &error {
        ExecutionError::RowBoundBreached {
            stratum,
            declared,
            pulled,
            mode,
        } => {
            assert_eq!(**stratum, iri(&ex(DOCS)));
            assert_eq!(*declared, 3, "the tightest promise that serves this call");
            assert_eq!(*pulled, 4, "and the probe row that falsifies it");
            assert_eq!(
                *mode,
                BoundMode::Subsuming {
                    declared: BindingPattern::from_code("ff"),
                    invoked: BindingPattern::from_code("fb"),
                },
                "the mode the three was read at is the needle-free one, and the \
                 attribution says it serves a call made with the needle bound"
            );
        }
        other => panic!("expected RowBoundBreached, got {other:?}"),
    }
    assert!(
        error.to_string().contains(
            "declares at most 3 rows per invocation under mode `ff`, which serves this \
             call under mode `fb`, and the read returned 4"
        ),
        "a host reading only the message is told which declaration to go and fix: \
         {error}"
    );

    // And the waist's own refusal over the same declaration, which reads the bound
    // through the same function and so has to name the same mode.
    let raised = admit_at_depth(&inverted, 9).expect_err("a depth above the coarser bound");
    match raised {
        AdmissionError::DepthBoundViolation {
            stratum,
            declared,
            requested,
            mode,
        } => {
            assert_eq!(*stratum, iri(&ex(DOCS)));
            assert_eq!(
                declared, 3,
                "the coarser mode's number, not the invoked one's"
            );
            assert_eq!(requested, 9);
            assert_eq!(
                mode,
                BoundMode::Subsuming {
                    declared: BindingPattern::from_code("ff"),
                    invoked: BindingPattern::from_code("fb"),
                },
            );
        }
        other => panic!("expected DepthBoundViolation, got {other:?}"),
    }
    assert_eq!(
        admit_at_depth(&inverted, 9)
            .expect_err("the message is the one a host reads")
            .to_string(),
        format!(
            "stratum {} declares depth 9, but the registry bounds it at 3 under mode \
             `ff`, which serves this call under mode `fb`",
            ex(DOCS)
        )
    );

    // The neighbouring depths that must still admit, so the coarser bound is a bound
    // and not a lock.
    for depth in 1..=3 {
        admit_at_depth(&inverted, depth)
            .unwrap_or_else(|error| panic!("depth {depth} is inside the bound: {error}"));
    }

    // THE CONTROL: the coarser mode removed, so nothing declares less than the invoked
    // mode's hundred. The same twenty rows are served, nothing is refused, and the
    // refusals above are therefore readings of the `ff` declaration rather than of
    // anything this fixture does to `fb`.
    let uninverted = registry(Shape::EvaluatorBounded, Modes::InvokedOnly, 100, 3, 20);
    assert_eq!(
        planned(&uninverted),
        (100, Some(100)),
        "with only the invoked mode declared, its own hundred is the bound"
    );
    assert_eq!(
        status(&uninverted),
        ProducerStatus::Exhausted { rows_emitted: 20 },
        "and the twenty rows it holds are a verified exhaustion, not a breach"
    );

    // And the honest neighbour under the inverted declaration: three rows behind a
    // bound of three is served and certified, so the refusal above is about the count
    // and not about the second mode existing.
    let honest = registry(Shape::EvaluatorBounded, Modes::Both, 100, 3, 3);
    assert_eq!(
        run(&honest).expect("an honest producer runs"),
        (3, ProducerStatus::Exhausted { rows_emitted: 3 }),
        "three rows behind the coarser mode's three is an exhaustion this layer can \
         verify"
    );
}

// ---------------------------------------------------------------------------
// C. A self-bounding producer is never asked for more than its mode declared
// ---------------------------------------------------------------------------

/// A producer that takes its own row request as an argument is handed the number its
/// INVOKED mode declared, and a producer that guards that number keeps its stratum.
///
/// This was the worst of the three. The request the layer writes into such a producer's
/// argument is `max(1, min(depth + 1, declared))`, and the `min` exists so the layer
/// never asks a producer to contradict its own registration — the shipped
/// nearest-neighbour relation refuses that against its configured guard rather than
/// returning a short answer as a complete one. Taken against the *unread* mode, the
/// `min` selected a hundred, the layer asked a producer that had declared three for
/// ten rows, and the guard refused: the stratum left the run as
/// `ExecutionFailed` and contributed nothing at all, while the single-mode control over
/// the identical relation was served.
///
/// So both are executed, and both must now be served identically.
#[test]
fn a_self_bounding_producer_is_asked_for_its_invoked_modes_number() {
    for modes in [Modes::Both, Modes::InvokedOnly] {
        let guarded = registry(Shape::SelfBounding, modes, 3, 100, 20);
        assert_eq!(
            planned(&guarded),
            (3, Some(3)),
            "the depth and the bound are the invoked mode's, so the request written \
             into the producer's own argument is a number it declared it can serve"
        );
        assert_eq!(
            status(&guarded),
            ProducerStatus::RowBoundReached { rank: 3 },
            "the producer served the read and the ending names its own declaration as \
             the stopper: the request landed ON the depth, so whether a further row \
             exists was not observable"
        );
    }

    // The same relation at a shallower depth, for both mode sets: a read the DEPTH
    // bounds rather than the declaration, which is the neighbouring case that proves
    // the request is not pinned to the declaration either.
    for modes in [Modes::Both, Modes::InvokedOnly] {
        let shallow = registry(Shape::SelfBounding, modes, 3, 100, 20);
        let stats = NoStatistics::new();
        let request = RetrievalRequest::complete(vec![needle()]);
        let mut planned = plan(&request, &shallow, &stats).expect("the needle plans");
        planned.stratum_depths.insert(iri(&ex(DOCS)), 2);
        let env = AdmissionEnvironment {
            registry: &shallow,
            statistics: &stats,
            fusion_profile: None,
        };
        let compiled = compile(&planned, &env).expect("a depth of two is inside the declaration");
        let execution = block_on(execute(&compiled, &shallow, &*common::empty_dataset()))
            .expect("the unit runs");
        assert_eq!(
            execution.statuses[&iri(&ex(DOCS))],
            ProducerStatus::DepthReached { rank: 2 },
            "at a depth of two the request is three — one row past it and still inside \
             the declaration — so the probe row arrives and the depth is the stopper"
        );
    }
}

// ---------------------------------------------------------------------------
// The emitted text carries the invoked mode's number
// ---------------------------------------------------------------------------

/// The number rendered into a self-bounding producer's argument is the invoked mode's
/// declaration, read off the emitted text.
///
/// The status assertions above say the producer accepted what it was handed; this says
/// which number that was, so a fix that merely widened the guard could not pass.
#[test]
fn the_emitted_request_is_the_invoked_modes_number_and_not_the_other_modes() {
    let stats = NoStatistics::new();
    let request = RetrievalRequest::complete(vec![needle()]);
    for modes in [Modes::Both, Modes::InvokedOnly] {
        let registry = registry(Shape::SelfBounding, modes, 3, 100, 20);
        let planned = plan(&request, &registry, &stats).expect("the needle plans");
        let env = AdmissionEnvironment {
            registry: &registry,
            statistics: &stats,
            fusion_profile: None,
        };
        let compiled = compile(&planned, &env).expect("the plan is admitted");
        let text = compiled.units[0].sparql();
        assert!(
            text.contains(&format!("\"3\"^^<{XSD_INTEGER}>")),
            "the producer is asked for the three rows its invoked mode declares: {text}"
        );
        assert!(
            !text.contains(&format!("\"100\"^^<{XSD_INTEGER}>")),
            "and never for the hundred its other mode declares: {text}"
        );
        assert!(
            !text.contains(&format!("\"4\"^^<{XSD_INTEGER}>")),
            "nor for the probe row its declaration leaves no room for: {text}"
        );
    }
}

// ---------------------------------------------------------------------------
// D. A tie between two declared modes is broken by the mode, not by its position
//    in the declaration
// ---------------------------------------------------------------------------

/// A registry holding one producer of `shape` that declares exactly `modes`, in that
/// order, and promises `rows` under every one of them.
///
/// The constant promise is the point: it puts two declared modes at the same count so
/// the tightest-bound rule cannot separate them, which is the only situation in which
/// the order they were declared in could possibly be read. It holds no rows, because
/// nothing here is executed — a tie is settled at the waist, before a producer is opened.
fn registry_declaring(
    shape: Shape,
    modes: Vec<BindingPattern>,
    rows: u64,
) -> PropertyFunctionRegistry {
    let arity = match shape {
        Shape::EvaluatorBounded => PfArity::new(1, 1),
        Shape::SelfBounding => PfArity::new(1, 2),
    };
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex(PRODUCER),
        Arc::new(ModedProducer {
            arity,
            modes,
            bound_rows: rows,
            free_rows: rows,
            emitted: Vec::new(),
            needle_position: 1,
            depth_position: match shape {
                Shape::EvaluatorBounded => None,
                Shape::SelfBounding => Some(2),
            },
        }),
        RankedDeclaration {
            stratum: purrdf_core::parse_iri(&ex(DOCS)).expect("fixture IRI"),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern::of_kind(TermKind::Literal),
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }],
            }],
            depth_placement: match shape {
                Shape::EvaluatorBounded => None,
                Shape::SelfBounding => Some(DepthPlacement {
                    position: 2,
                    datatype: XSD_INTEGER.to_owned(),
                }),
            },
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            // This mock answers from a list it holds entire: its search names
            // every row that was due to it and ranks them by the order it was
            // given, so the exhaustive declaration is the true one. It is
            // stated rather than left out, because there is no default.
            fidelity: RankFidelity::EXACT,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            mandatory: false,
        },
    );
    registry
}

/// **Two declared modes promising the same count resolve to the same one whichever
/// order they were registered in.**
///
/// The bound is the tightest promise any declared mode that serves the call makes, and
/// where two such modes make the *same* promise the count is settled but the mode is
/// not — and the mode is what the refusals name, so something has to choose. The rule is
/// that the choice is a function of the declaration and not of its position in a list:
/// the invoked mode wins where it is among the tied candidates, and otherwise the lowest
/// `BindingPattern` in its own total order does.
///
/// Order-independence was claimed and never executed. A tightest-bound search that
/// compared counts alone would return whichever tied mode it happened to reach first, so
/// registering the same two promises the other way round would have changed the mode in
/// a refusal message while nothing about the producer changed — a diagnostic that is a
/// fact about a registration order rather than about a registration.
#[test]
fn a_tie_between_two_modes_resolves_the_same_way_in_either_declaration_order() {
    const ROWS: u64 = 4;
    // The mode this fixture's plan invokes — the needle at position 1 bound, as
    // everywhere in this file — and the coarser mode that leaves it free. Both serve the
    // call: the coarser one demands strictly less, and the invoked one serves itself.
    let invoked = BindingPattern::from_code("fb");
    let coarser = BindingPattern::from_code("ff");

    for (order, modes) in [
        ("the invoked mode declared first", vec![invoked, coarser]),
        ("the coarser mode declared first", vec![coarser, invoked]),
    ] {
        let registry = registry_declaring(Shape::EvaluatorBounded, modes, ROWS);
        // The premise, asserted rather than assumed: this fixture really does promise the
        // same count under both modes, so there really is a tie to break.
        assert_eq!(
            planned(&registry),
            (
                u32::try_from(ROWS).expect("a fixture count fits"),
                Some(ROWS)
            ),
            "both declared modes promise {ROWS}, so the count is the same either way \
             ({order})"
        );

        let error = admit_at_depth(&registry, u32::try_from(ROWS).expect("fits") + 1)
            .expect_err("a depth one past the declaration is refused");
        match &error {
            AdmissionError::DepthBoundViolation { mode, .. } => assert_eq!(
                *mode,
                BoundMode::Invoked { mode: invoked },
                "the invoked mode is among the tied candidates, so it is the one named, \
                 with {order}"
            ),
            other => panic!("expected DepthBoundViolation, got {other:?}"),
        }
        assert_eq!(
            error.to_string(),
            format!(
                "stratum {} declares depth 5, but the registry bounds it at 4 under mode \
                 `fb`",
                ex(DOCS)
            ),
            "and the sentence a host reads is the same sentence, with {order}"
        );
    }

    // The other rung of the same rule: a tie in which NEITHER candidate is the invoked
    // mode, so the count and the invoked-mode preference both fall silent and the
    // `BindingPattern` order alone decides.
    //
    // That rung needs an invocation binding TWO positions, because the modes that serve
    // a call are the declared ones whose bound set is a subset of the invocation's, and a
    // one-binding invocation has exactly one such mode that is not itself. So this half
    // runs over the self-bounding shape, whose invocation binds the needle at 1 and the
    // row request at 2 — and the two declared modes each drop one of the two.
    let self_invoked = BindingPattern::from_code("fbb");
    let lower = BindingPattern::from_code("fbf");
    let higher = BindingPattern::from_code("ffb");
    assert!(
        lower < higher,
        "the expectation below is the lower of the two in BindingPattern's own order"
    );
    for (order, modes) in [
        ("lowest declared first", vec![lower, higher]),
        ("lowest declared last", vec![higher, lower]),
    ] {
        let registry = registry_declaring(Shape::SelfBounding, modes, ROWS);
        assert_eq!(
            planned(&registry),
            (
                u32::try_from(ROWS).expect("a fixture count fits"),
                Some(ROWS)
            ),
            "both declared modes serve this call and promise {ROWS}, so this is a tie \
             and not a choice between two counts ({order})"
        );
        let error = admit_at_depth(&registry, u32::try_from(ROWS).expect("fits") + 1)
            .expect_err("a depth one past the declaration is refused");
        match &error {
            AdmissionError::DepthBoundViolation { mode, .. } => assert_eq!(
                *mode,
                BoundMode::Subsuming {
                    declared: lower,
                    invoked: self_invoked,
                },
                "with no invoked mode among the tied candidates the lowest one is named, \
                 with {order}"
            ),
            other => panic!("expected DepthBoundViolation, got {other:?}"),
        }
        assert_eq!(
            error.to_string(),
            format!(
                "stratum {} declares depth 5, but the registry bounds it at 4 under mode \
                 `fbf`, which serves this call under mode `fbb`",
                ex(DOCS)
            ),
            "and again one sentence, whichever order they were registered in ({order})"
        );
    }
}

// ---------------------------------------------------------------------------
// A tiny executor, so the tests need no runtime
// ---------------------------------------------------------------------------

/// Drive `future` to completion on the current thread.
///
/// `execute` is asynchronous as a stage contract and never actually pends, so a
/// one-poll loop over the no-op waker is the whole executor these tests need.
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}
