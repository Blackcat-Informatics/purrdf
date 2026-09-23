// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! The fused read taken on demand, against the same read materialised.
//!
//! [`search`] reads every stratum it rendered as one invocation held open and read a
//! row per pull; the hand-composed pipeline over [`execute`] materialises each
//! stratum at its planned depth before its first row is readable. Both give the
//! same answer. These benches put the two side by side over the three shapes that
//! decide what the difference is worth:
//!
//! * `intersecting` — two strata sharing a block and naming the same candidates, so
//!   the fusion certifies at its sixth rank of a deep plan: the on-demand read
//!   produces six rows per stratum where the materialised one produces the plan;
//! * `disjoint` — the same block and no candidate named twice, with nothing
//!   answerable, so the fusion drains both streams: the two reads produce the same
//!   rows, and the difference is the price of reading them one pull at a time;
//! * `disjoint_asking` — the same rows with both producers answering exclusion
//!   lookups, so the fusion stops at the threshold crossing and the on-demand read
//!   stops there with it.
//!
//! Each group runs at two corpus sizes, because the first and third shapes' on-demand
//! cost is flat in the corpus and the materialised cost is not.
//!
//! Report-only, per this repository's rule: benches exist so a later change has a
//! number to move, never so a speedup can be asserted. The deterministic
//! counterparts — rows produced and producer-reported work, exactly, against the
//! materialised control — are asserted in `tests/multimodal_read_bound.rs`.

use std::collections::BTreeMap;
use std::future::Future;
use std::hint::black_box;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_retrieval::{
    AdmissionEnvironment, CandidateDomains, DecayRule, DomainTag, DuplicatePolicy, Fixed,
    FusionProfile, Iri, RECIP_K, RankFidelity, RankedStreamAdapter, RequestTerm, RetrievalRequest,
    Statistics, Term, TopK, compile, execute, fuse, plan, search,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, EvalError, ExclusionBasis, PfArgs, PfArity, PfCursor, PfRow,
    PropertyFunction, PropertyFunctionRegistry, RankedDeclaration, RequestFacet, TermKind,
    TermPattern, TermPlacement, Volatility,
};

/// The fixture namespace. A bench mints no vocabulary of its own, and a
/// reserved-for-documentation authority is the only one it may put in a term.
fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("fixture IRIs are valid")
}

/// A single-threaded executor; nothing here ever pends.
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

/// The flattened argument position every producer here projects its candidate from.
const CANDIDATE_POSITION: usize = 0;

/// One shape of the two producers.
#[derive(Clone, Copy)]
struct Shape {
    name: &'static str,
    /// Whether the two producers name the same candidates.
    intersecting: bool,
    /// Whether they answer exclusion lookups.
    asking: bool,
}

const SHAPES: [Shape; 3] = [
    Shape {
        name: "intersecting",
        intersecting: true,
        asking: false,
    },
    Shape {
        name: "disjoint",
        intersecting: false,
        asking: false,
    },
    Shape {
        name: "disjoint_asking",
        intersecting: false,
        asking: true,
    },
];

/// A ranked producer holding `rows` candidates under `prefix`, minted as pulled.
struct Producer {
    prefix: &'static str,
    rows: u64,
    modes: Vec<BindingPattern>,
}

impl PropertyFunction for Producer {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        if mode.is_bound(CANDIDATE_POSITION) {
            1
        } else {
            self.rows
        }
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let bound: Vec<Option<TermValue>> =
            args.flattened().map(Option::<&TermValue>::cloned).collect();
        // A bound candidate is the exclusion lookup: answered from the minted
        // spelling, never by scanning.
        if let Some(candidate) = bound[CANDIDATE_POSITION].clone() {
            let held = matches!(&candidate, TermValue::Iri(text)
                if text.strip_prefix(&ex(self.prefix)).and_then(|rest| rest.strip_prefix("entity"))
                    .and_then(|index| index.parse::<u64>().ok())
                    .is_some_and(|index| index < self.rows));
            return Ok(Box::new(Rows {
                prefix: self.prefix,
                next: 0,
                end: u64::from(held),
                bound,
            }));
        }
        Ok(Box::new(Rows {
            prefix: self.prefix,
            next: 0,
            end: self.rows,
            bound,
        }))
    }
}

/// The cursor behind [`Producer`]: candidates `next..end`, minted as pulled.
struct Rows {
    prefix: &'static str,
    next: u64,
    end: u64,
    bound: Vec<Option<TermValue>>,
}

impl PfCursor for Rows {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.next >= self.end {
            return Ok(None);
        }
        let minted = [
            TermValue::iri(format!("{}entity{:09}", ex(self.prefix), self.next)),
            TermValue::iri(format!("{}score{:09}", ex(self.prefix), self.next)),
        ];
        self.next += 1;
        Ok(Some(
            minted
                .iter()
                .enumerate()
                .map(|(position, value)| {
                    self.bound
                        .get(position)
                        .cloned()
                        .flatten()
                        .unwrap_or_else(|| value.clone())
                })
                .collect(),
        ))
    }
}

fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

fn registry(shape: Shape, rows: u64) -> PropertyFunctionRegistry {
    let arity = PfArity::new(1, 1);
    let block = DomainTag::parse(&ex("domain/shared")).expect("a valid tag");
    let prefixes = if shape.intersecting {
        ["shared/", "shared/"]
    } else {
        ["left/", "right/"]
    };
    let mut registry = PropertyFunctionRegistry::new();
    for ((predicate, stratum), prefix) in ["title", "body"].into_iter().zip(strata()).zip(prefixes)
    {
        let mut modes = vec![arity.all_free_mode()];
        if shape.asking {
            modes.push(BindingPattern::from_bound_positions(
                arity.total(),
                [CANDIDATE_POSITION],
            ));
        }
        registry.register_ranked(
            ex(&format!("pf/{predicate}")),
            Arc::new(Producer {
                prefix,
                rows,
                modes,
            }),
            RankedDeclaration {
                stratum: purrdf_core::parse_iri(stratum.as_str()).expect("a valid IRI"),
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
                depth_placement: None,
                candidate_position: CANDIDATE_POSITION,
                duplicates: DuplicatePolicy::Unique,
                fidelity: RankFidelity::EXACT,
                domains: CandidateDomains::within([block.clone()]),
                block_position: None,
                exclusion: if shape.asking {
                    ExclusionBasis::Membership
                } else {
                    ExclusionBasis::Unavailable
                },
                mandatory: false,
            },
        );
    }
    registry
}

fn request() -> RetrievalRequest {
    let terms = ["title", "body"]
        .into_iter()
        .map(|predicate| RequestTerm::Lexical {
            text: "quick brown fox".to_owned(),
            language: Some("en".to_owned()),
            predicate: Some(iri(&ex(predicate))),
        })
        .collect();
    RetrievalRequest::bounded(terms, TopK::new(5))
}

/// Statistics that narrow nothing: each stratum holds what its producer declares.
struct Cardinalities(BTreeMap<Iri, u64>);

impl Statistics for Cardinalities {
    fn source(&self) -> &str {
        "example-statistics"
    }

    fn revision(&self) -> &str {
        "r1"
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        self.0.get(predicate).copied()
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

fn statistics(rows: u64) -> Cardinalities {
    let mut cardinalities = BTreeMap::new();
    for stratum in strata() {
        cardinalities.insert(stratum, rows);
    }
    for predicate in ["title", "body"] {
        cardinalities.insert(iri(&ex(predicate)), rows);
    }
    Cardinalities(cardinalities)
}

fn profile() -> FusionProfile {
    FusionProfile::with_decay(
        strata()
            .into_iter()
            .map(|stratum| (stratum, Fixed::ONE))
            .collect(),
        DecayRule::ReciprocalRank {
            k: u32::try_from(RECIP_K).expect("the smoothing constant fits"),
        },
    )
    .expect("the fixture profile is valid")
}

/// The read `search` takes: every stratum on demand.
fn on_demand(
    registry: &PropertyFunctionRegistry,
    statistics: &Cardinalities,
    dataset: &RdfDataset,
) -> usize {
    let profile = profile();
    let env = AdmissionEnvironment {
        registry,
        statistics,
        fusion_profile: Some(&profile),
    };
    block_on(search(
        &request(),
        registry,
        statistics,
        dataset,
        &env,
        &profile,
    ))
    .expect("the fixture searches")
    .rows
    .len()
}

/// The same answer, read materialised: `execute` at the planned depths, then `fuse`.
fn materialised(
    registry: &PropertyFunctionRegistry,
    statistics: &Cardinalities,
    dataset: &RdfDataset,
) -> usize {
    let profile = profile();
    let env = AdmissionEnvironment {
        registry,
        statistics,
        fusion_profile: Some(&profile),
    };
    let planned = plan(&request(), registry, statistics).expect("the fixture plans");
    let compiled = compile(&planned, &env).expect("the fixture compiles");
    let execution = block_on(execute(&compiled, registry, dataset)).expect("it executes");
    let streams = execution
        .streams
        .into_iter()
        .map(|stream| {
            let adapter =
                RankedStreamAdapter::new(stream.stream, stream.contract, &profile, &stream.stratum)
                    .expect("both strata are weighted")
                    .with_plan_id(stream.plan_id)
                    .with_fused_bound(stream.fused_bound)
                    .with_attestation(stream.attestation);
            (stream.stratum, adapter)
        })
        .collect();
    block_on(fuse::<RankedStreamAdapter<'_>, Term>(
        streams,
        &profile,
        compiled.fused_bound,
    ))
    .expect("the materialised streams fuse")
    .rows
    .len()
}

fn reads(c: &mut Criterion) {
    let dataset = RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid");
    for shape in SHAPES {
        let mut group = c.benchmark_group(format!("on_demand_read/{}", shape.name));
        for rows in [400_u64, 4_000] {
            let registry = registry(shape, rows);
            let statistics = statistics(rows);
            group.bench_with_input(BenchmarkId::new("on_demand", rows), &rows, |b, _| {
                b.iter(|| black_box(on_demand(&registry, &statistics, &dataset)));
            });
            group.bench_with_input(BenchmarkId::new("materialised", rows), &rows, |b, _| {
                b.iter(|| black_box(materialised(&registry, &statistics, &dataset)));
            });
        }
        group.finish();
    }
}

criterion_group!(benches, reads);
criterion_main!(benches);
