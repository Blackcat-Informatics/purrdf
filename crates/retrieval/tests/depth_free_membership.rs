// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A declared exclusion basis is admitted only against a mode an exclusion lookup
//! is really invoked in.
//!
//! Every exclusion lookup frees the depth a producer declares
//! ([`RankedDeclaration::exclusion_lookup_mode`]): a producer handed a depth
//! answers *is this candidate among your best n*, whose absences are not
//! exclusions. So a candidate-bound point mode that also binds the depth is a mode
//! no lookup is ever invoked in, and a basis admitted on its strength is a promise
//! the first search breaks — every stratum's lookup fails to prepare, and the
//! search answers nothing. Registration is where that is caught.
//!
//! Every refusal below is executed beside the neighbour that differs from it in
//! the one mode under test and is admitted, and every admission is then *used*: a
//! search runs, the producer counts the lookups it was asked, and the answer is
//! compared with the one the same producer gives under
//! [`ExclusionBasis::Unavailable`], which asks none. A lookup count of zero on the
//! admitted arm would be a basis silently dropped, and it is asserted against.
//!
//! Fixtures use `example.org` throughout; every IRI below is fixture
//! configuration, never a minted vocabulary.

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use pretty_assertions::assert_eq;
use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, DuplicatePolicy, Fixed,
    FusionProfile, Iri, RECIP_K, RankFidelity, RequestTerm, RetrievalRequest, SearchResult,
    Statistics, TopK, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, DepthPlacement, EvalError, ExclusionBasis, IndexGeneration, PfArgs, PfArity,
    PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, RankedDeclaration, RequestFacet,
    ServiceLevel, TermKind, TermPattern, TermPlacement, Volatility,
};

/// How many rows each producer ranks.
const ROWS: u64 = 400;

/// The answer size every search here asks for.
const TOP_K: TopK = TopK::new(5);

/// The two request terms, one per stratum.
const PREDICATES: [&str; 2] = ["title", "body"];

/// Each stratum's candidates are minted under its own prefix, so no candidate is
/// held by both: every lookup of the other stratum's candidate is an exclusion.
const PREFIXES: [&str; 2] = ["left/", "right/"];

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

fn producer_iri(predicate: &str) -> String {
    ex(&format!("pf/{predicate}"))
}

fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

/// A minimal single-threaded executor. The producers never pend.
fn block_on<F: Future>(future: F) -> F::Output {
    struct ParkWaker(std::thread::Thread);
    impl Wake for ParkWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ParkWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

/// Where a producer takes its per-stratum depth, if it takes one at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    /// `( ?candidate ) <pf> ( needle depth )`: the depth at position 2.
    Depth,
    /// `( ?candidate ) <pf> ( needle )`: no depth anywhere.
    NoDepth,
}

impl Shape {
    const fn arity(self) -> PfArity {
        match self {
            Self::Depth => PfArity::new(1, 2),
            Self::NoDepth => PfArity::new(1, 1),
        }
    }
}

/// A producer ranking `{prefix}entity{index:06}` for every index below [`ROWS`],
/// whose candidate-bound call answers from the candidate IRI's own prefix.
///
/// Every invocation's mode is recorded, so a test reads exactly how many lookups
/// the search asked and in which mode, from the producer's side.
struct Producer {
    arity: PfArity,
    modes: Vec<BindingPattern>,
    prefix: &'static str,
    asked: Arc<Mutex<Vec<String>>>,
}

impl PropertyFunction for Producer {
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
        if mode.is_bound(0) { 1 } else { ROWS }
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.asked
            .lock()
            .expect("the fixture's record is never poisoned")
            .push(args.mode().code());
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        // Every free position but the candidate's is echoed back as a fixture
        // value: nothing reads it, and a free position must still carry a term.
        let fill = |position: usize| {
            bound[position]
                .clone()
                .unwrap_or_else(|| TermValue::iri(ex("unread")))
        };
        let tail: Vec<TermValue> = (1..bound.len()).map(fill).collect();
        let row = |candidate: TermValue| {
            let mut row = vec![candidate];
            row.extend(tail.iter().cloned());
            row
        };
        let rows: Vec<PfRow> = match bound[0].clone() {
            Some(candidate) => {
                let held = matches!(
                    &candidate,
                    TermValue::Iri(held) if held.as_str().starts_with(&ex(self.prefix))
                );
                if held {
                    vec![row(candidate)]
                } else {
                    Vec::new()
                }
            }
            None => (0..ROWS)
                .map(|index| {
                    row(TermValue::iri(format!(
                        "{}entity{index:06}",
                        ex(self.prefix)
                    )))
                })
                .collect(),
        };
        Ok(Box::new(Rows(rows.into_iter())))
    }
}

struct Rows(std::vec::IntoIter<PfRow>);

impl PfCursor for Rows {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.0.next())
    }

    fn generation(&self) -> IndexGeneration {
        IndexGeneration::Undeclared
    }

    fn service_level(&self) -> ServiceLevel {
        ServiceLevel::Undeclared
    }
}

/// Register both strata's producers under `shape`, `modes` and `exclusion`.
///
/// Both declare the one shared domain, so neither stratum's absence of a candidate
/// is settled by a declaration: only a lookup can settle it early.
fn registry(
    shape: Shape,
    modes: &[&str],
    exclusion: ExclusionBasis,
) -> (PropertyFunctionRegistry, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let shared = DomainTag::parse(&ex("domain/shared")).expect("fixture domain tags are IRIs");
    let mut registry = PropertyFunctionRegistry::new();
    for ((predicate, stratum), prefix) in PREDICATES.into_iter().zip(strata()).zip(PREFIXES) {
        registry.register_ranked(
            producer_iri(predicate),
            Arc::new(Producer {
                arity: shape.arity(),
                modes: modes
                    .iter()
                    .map(|code| BindingPattern::from_code(code))
                    .collect(),
                prefix,
                asked: Arc::clone(&asked),
            }),
            RankedDeclaration {
                stratum: purrdf_core::parse_iri(stratum.as_str()).expect("fixture IRI"),
                accepted_terms: vec![AcceptedTerm {
                    pattern: TermPattern {
                        kind: TermKind::Literal,
                        datatype: None,
                        language: Some("en".to_owned()),
                        predicate: Some(ex(predicate)),
                    },
                    placements: vec![TermPlacement {
                        facet: RequestFacet::Value,
                        position: 1,
                        datatype: None,
                    }],
                }],
                depth_placement: match shape {
                    Shape::Depth => Some(DepthPlacement {
                        position: 2,
                        datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                    }),
                    Shape::NoDepth => None,
                },
                candidate_position: 0,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                domains: CandidateDomains::within([shared.clone()]),
                block_position: None,
                exclusion,
                mandatory: false,
            },
        );
    }
    (registry, asked)
}

/// Serialises every swap of the process-wide panic hook in this binary.
static HOOK_LOCK: Mutex<()> = Mutex::new(());

/// The panic message [`registry`] raised, or `None` where it registered.
fn refusal(shape: Shape, modes: &[&str], exclusion: ExclusionBasis) -> Option<String> {
    let _hook = HOOK_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let (registry, _) = registry(shape, modes, exclusion);
        for predicate in PREDICATES {
            assert_eq!(
                registry
                    .ranked_declaration(&producer_iri(predicate))
                    .expect("the producer registered")
                    .exclusion,
                exclusion,
                "an admitted registration stores the basis it was handed"
            );
        }
    }));
    std::panic::set_hook(previous);
    outcome.err().map(|payload| {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_else(|| "a panic with no message".to_owned())
    })
}

struct FixtureStatistics;

impl Statistics for FixtureStatistics {
    fn source(&self) -> &'static str {
        "example-statistics"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, _predicate: &Iri) -> Option<u64> {
        Some(ROWS)
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

fn request() -> RetrievalRequest {
    RetrievalRequest::bounded(
        PREDICATES
            .into_iter()
            .map(|predicate| RequestTerm::Lexical {
                text: "quick brown fox".to_owned(),
                language: Some("en".to_owned()),
                predicate: Some(iri(&ex(predicate))),
            })
            .collect(),
        TOP_K,
    )
}

fn profile() -> FusionProfile {
    FusionProfile::with_decay(
        strata()
            .into_iter()
            .map(|stratum| (stratum, Fixed::ONE))
            .collect(),
        DecayRule::ReciprocalRank { k: RECIP_K as u32 },
    )
    .expect("the fixture profile is valid")
}

fn empty_dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid")
}

/// What one search answered, and what its producers were asked.
#[derive(Debug, PartialEq, Eq)]
struct Searched {
    /// The fused entities, in answer order.
    entities: Vec<String>,
    /// How many invocations each mode was asked, by mode code, ascending.
    asked: Vec<(String, usize)>,
}

/// Register under `shape`, `modes` and `exclusion`, and run the fixture request.
fn searched(shape: Shape, modes: &[&str], exclusion: ExclusionBasis) -> Searched {
    let (registry, asked) = registry(shape, modes, exclusion);
    let statistics = FixtureStatistics;
    let profile = profile();
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: Some(&profile),
    };
    let dataset = empty_dataset();
    let result: SearchResult = block_on(search(
        &request(),
        &registry,
        &statistics,
        dataset.as_ref(),
        &env,
        &profile,
    ))
    .expect("the fixture search answers");
    let entities = result
        .rows
        .iter()
        .map(|row| row.entity.as_str().to_owned())
        .collect();
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for mode in asked
        .lock()
        .expect("the fixture's record is never poisoned")
        .iter()
    {
        *counts.entry(mode.clone()).or_default() += 1;
    }
    Searched {
        entities,
        asked: counts.into_iter().collect(),
    }
}

fn asked(pairs: &[(&str, usize)]) -> Vec<(String, usize)> {
    pairs
        .iter()
        .map(|(code, count)| ((*code).to_owned(), *count))
        .collect()
}

/// The five entities every search here answers, as the fusion spells a term:
/// both strata's first three, fused under equal weights, cut to five.
fn expected_entities() -> Vec<String> {
    [
        "left/entity000000",
        "right/entity000000",
        "left/entity000001",
        "right/entity000001",
        "left/entity000002",
    ]
    .into_iter()
    .map(|suffix| format!("<{}>", ex(suffix)))
    .collect()
}

/// **A basis whose only point mode binds the depth is refused at registration,
/// naming the relation, its modes and the freed depth.**
///
/// Beside it, executed: the neighbour whose point mode differs only in leaving
/// the depth free is admitted, and the refused producer is admitted too when it
/// declares no basis — so the refusal is about the basis resting on a mode no
/// lookup is invoked in, not about the producer.
#[test]
fn a_point_mode_binding_the_depth_is_refused_at_registration() {
    let refused = refusal(Shape::Depth, &["fbb", "bbb"], ExclusionBasis::Membership)
        .expect("a basis no lookup can be invoked under must be refused where it is declared");
    for part in [
        "ranked declaration for <http://example.org/pf/title>",
        "declares an exclusion basis of membership",
        "declares no access mode that binds its candidate position (0), that leaves its \
         per-stratum depth position (2) free, with a row bound of one",
        "it declares [fbb, bbb]",
        "every exclusion lookup frees it",
        "a lookup is invoked no more bound than `bbf`",
        "a mode binding position 2 serves none of them",
        "ExclusionBasis::Unavailable",
    ] {
        assert!(
            refused.contains(part),
            "the refusal names `{part}`: {refused}"
        );
    }

    assert_eq!(
        refusal(Shape::Depth, &["fbb", "bbf"], ExclusionBasis::Membership),
        None,
        "the neighbour whose point mode frees the depth is admitted"
    );
    assert_eq!(
        refusal(Shape::Depth, &["fbb", "bff"], ExclusionBasis::Membership),
        None,
        "and so is one whose point mode binds the candidate alone: a lookup binding the \
         needle too is still served by it"
    );
    assert_eq!(
        refusal(Shape::Depth, &["fbb", "bbb"], ExclusionBasis::Unavailable),
        None,
        "the refused producer is admitted when it declares no basis"
    );
}

/// **The admitted neighbour is looked up, and answers what the basis-less
/// producer answers.**
///
/// Every lookup the fusion asks runs in the depth-free point mode `bbf` — 130 of
/// them, the needle bound and the depth free, beside the two ranked reads — and
/// the fused answer is identical to the one the same producer
/// gives declaring no basis, which asks no lookup at all. The lookup count is
/// the oracle: a basis admitted and then dropped would read `bbf` zero times.
#[test]
fn a_depth_free_point_mode_is_admitted_and_looked_up() {
    let unavailable = searched(Shape::Depth, &["fbb", "bbb"], ExclusionBasis::Unavailable);
    assert_eq!(
        unavailable,
        Searched {
            entities: expected_entities(),
            asked: asked(&[("fbb", 2)]),
        },
        "without a basis each stratum is read once, ranked, and never looked up"
    );

    let membership = searched(Shape::Depth, &["fbb", "bbf"], ExclusionBasis::Membership);
    assert_eq!(
        membership,
        Searched {
            entities: expected_entities(),
            asked: asked(&[("bbf", 130), ("fbb", 2)]),
        },
        "with the depth-free point mode declared, the lookups run in exactly that mode — \
         the needle bound, the depth free — and change no answer"
    );
}

/// **A producer placing no depth keeps the rule it always had.**
///
/// Nothing is freed, so a point mode binding every other position is admitted,
/// and it is the mode the lookups run in. Its answer is the basis-less one.
#[test]
fn without_a_depth_the_candidate_bound_point_mode_is_admitted_and_used() {
    assert_eq!(
        refusal(Shape::NoDepth, &["fb", "bb"], ExclusionBasis::Membership),
        None,
        "with no depth placed, the candidate-bound point mode is all a basis needs"
    );

    let unavailable = searched(Shape::NoDepth, &["fb", "bb"], ExclusionBasis::Unavailable);
    let membership = searched(Shape::NoDepth, &["fb", "bb"], ExclusionBasis::Membership);
    assert_eq!(
        unavailable,
        Searched {
            entities: expected_entities(),
            asked: asked(&[("fb", 2)]),
        },
        "without a basis nothing is looked up"
    );
    assert_eq!(
        membership,
        Searched {
            entities: expected_entities(),
            asked: asked(&[("bb", 130), ("fb", 2)]),
        },
        "with one, the lookups run in the declared point mode and change no answer"
    );
}

/// The lookup side reads the same derivation: a lookup of a producer placing a
/// depth frees it whatever it is asked to supply, and binds the candidate.
#[test]
fn the_lookup_shape_frees_the_depth_and_binds_the_candidate() {
    let (registry, _) = registry(Shape::Depth, &["fbb", "bbf"], ExclusionBasis::Membership);
    let declaration = registry
        .ranked_declaration(&producer_iri("title"))
        .expect("the producer registered");
    assert_eq!(
        declaration.exclusion_lookup_mode(3, |_| true).code(),
        "bbf",
        "every position supplied: the depth is still free"
    );
    assert_eq!(
        declaration.exclusion_lookup_mode(3, |_| false).code(),
        "bff",
        "nothing supplied: the candidate is still bound"
    );
    let (registry, _) = registry_no_depth();
    let declaration = registry
        .ranked_declaration(&producer_iri("title"))
        .expect("the producer registered");
    assert_eq!(
        declaration.exclusion_lookup_mode(2, |_| true).code(),
        "bb",
        "with no depth placed, nothing supplied is freed"
    );
}

fn registry_no_depth() -> (PropertyFunctionRegistry, Arc<Mutex<Vec<String>>>) {
    registry(Shape::NoDepth, &["fb", "bb"], ExclusionBasis::Membership)
}
