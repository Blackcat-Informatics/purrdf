// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One request, two real ranked producers, one fused answer.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p purrdf-retrieval --example fused_search
//! ```
//!
//! The host below indexes the same three documents twice — once over
//! `ex:title`, once over `ex:body` — registers each index as its own ranked
//! producer under its own stratum, and asks one request that reaches both. The
//! printed answer carries the fused ranking, the per-stratum provenance behind
//! every row, every producer's terminal status, and the one request term nothing
//! in this registry accepts.
//!
//! Every IRI here is the host's own `example.org` vocabulary. PurRDF mints none,
//! and there is no default producer, stratum or weight to fall back on.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue, parse_iri};
use purrdf_retrieval::{
    AdmissionEnvironment, Fixed, FusionProfile, Iri, ProducerStatus, RequestTerm, RetrievalRequest,
    SearchResult, Statistics, Term, TopK, search,
};
use purrdf_sparql_eval::PropertyFunctionRegistry;
use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};

/// The predicate the title corpus is indexed over.
const TITLE: &str = "https://example.org/title";
/// The predicate the body corpus is indexed over.
const BODY: &str = "https://example.org/body";
/// The IRI this host registers the title producer under.
const TITLE_PRODUCER: &str = "https://example.org/pf/title-search";
/// The IRI this host registers the body producer under.
const BODY_PRODUCER: &str = "https://example.org/pf/body-search";
/// The stratum the title producer ranks within.
const TITLE_STRATUM: &str = "https://example.org/stratum/title";
/// The stratum the body producer ranks within.
const BODY_STRATUM: &str = "https://example.org/stratum/body";
/// The reciprocal-rank smoothing constant this host fuses under.
const K: u32 = 60;
/// How many fused rows this host wants. Fused enumeration is top-k by
/// construction, so the bound is stated rather than defaulted.
const TOP_K: TopK = TopK::new(10);

/// The corpus, as `(subject local name, title, body)`.
///
/// The two fields deliberately disagree: `doc-3` is a strong title match and
/// holds none of the body needle's terms, which is what makes "this row came
/// from one producer and that one from both" an observation rather than a
/// coincidence.
const CORPUS: [(&str, &str, &str); 3] = [
    (
        "doc-1",
        "black cat manual",
        "the cat sleeps on the warm mat",
    ),
    (
        "doc-2",
        "grey dog manual",
        "the cat and the dog share a mat",
    ),
    (
        "doc-3",
        "cat and mat catalogue",
        "a manual for folding paper",
    ),
];

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("the host's IRIs are valid")
}

/// The dataset every stage runs against: one `title` and one `body` triple per
/// document, in the default graph.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let title = builder.intern_iri(TITLE);
    let body = builder.intern_iri(BODY);
    for (local, title_text, body_text) in CORPUS {
        let subject = builder.intern_iri(&format!("https://example.org/{local}"));
        let title_literal = builder.intern_literal(RdfLiteral::simple(title_text));
        let body_literal = builder.intern_literal(RdfLiteral::simple(body_text));
        builder.push_quad(subject, title, title_literal, None);
        builder.push_quad(subject, body, body_literal, None);
    }
    builder.freeze().expect("the fixture dataset is valid")
}

/// Build a single-predicate index over `data`.
///
/// One predicate and one untagged, default-graph corpus is exactly one
/// partition, which is the condition `TextSearchRelation::ranked_declaration`
/// requires before it will claim a ranked order.
fn index(data: &RdfDataset, predicate: &str) -> TextIndex {
    let config = TextIndexConfig::new(vec![TermValue::iri(predicate)], GraphSelector::Any)
        .expect("one IRI predicate is a well-formed configuration");
    TextIndex::from_dataset(data, &config).expect("the index builds over the fixture dataset")
}

/// Register both indexes as ranked producers, each under the host's own
/// producer IRI and stratum, using the declaration the relation hands out.
fn registry(data: &RdfDataset) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    for (producer, stratum, predicate) in [
        (TITLE_PRODUCER, TITLE_STRATUM, TITLE),
        (BODY_PRODUCER, BODY_STRATUM, BODY),
    ] {
        let relation = TextSearchRelation::new(Arc::new(index(data, predicate)));
        let declaration = relation
            .ranked_declaration(
                parse_iri(stratum).expect("the host's stratum IRI is valid"),
                Some(predicate.to_owned()),
            )
            .expect("a single-partition index declares a ranked order");
        registry.register_ranked(producer, Arc::new(relation), declaration);
    }
    registry
}

/// The request: one needle per indexed field, plus a seed this registry has no
/// producer for.
///
/// The seed is deliberate. A request may name a modality no registered producer
/// accepts, and the answer says so per term rather than letting it vanish.
fn request() -> RetrievalRequest {
    RetrievalRequest::from_terms(vec![
        RequestTerm::Lexical {
            text: "cat manual".to_owned(),
            language: None,
            predicate: Some(iri(TITLE)),
        },
        RequestTerm::Lexical {
            text: "cat mat".to_owned(),
            language: None,
            predicate: Some(iri(BODY)),
        },
        RequestTerm::EntitySeed {
            entity: Term::new("<https://example.org/doc-1>"),
        },
    ])
}

/// Unit weights for both strata, `K` smoothing, and room for one candidate to
/// surface in both.
fn profile() -> FusionProfile {
    let mut weights = BTreeMap::new();
    weights.insert(iri(TITLE_STRATUM), Fixed::ONE);
    weights.insert(iri(BODY_STRATUM), Fixed::ONE);
    FusionProfile::new(weights, K, 2).expect("the host's profile is valid")
}

/// A statistics provider that reports nothing.
///
/// Both producers declare a finite row bound measured from their own frozen
/// index, so there is no unbounded declaration for a cardinality to bound. A
/// provider that invented one would put a number into the plan that no
/// measurement supports.
struct NoStatistics;

impl Statistics for NoStatistics {
    fn source(&self) -> &'static str {
        "example-statistics"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, _predicate: &Iri) -> Option<u64> {
        None
    }

    fn selectivity(&self, _predicate: &Iri, _term: &RequestTerm) -> Option<f64> {
        None
    }
}

/// Drive one future to completion on this thread.
///
/// The ladder's futures are awaited in a single task and never cross a thread
/// boundary, so a parking waker is the whole runtime they need; a host that
/// already has an executor awaits `search` on that instead.
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

/// The stratum IRI without the host's shared prefix, so the printed table reads.
fn short(stratum: &Iri) -> &str {
    stratum
        .as_str()
        .rsplit_once('/')
        .map_or_else(|| stratum.as_str(), |(_, tail)| tail)
}

fn report(result: &SearchResult) {
    println!("fused ranking (top {} requested)", TOP_K.get());
    for (position, row) in result.rows.iter().enumerate() {
        let provenance = row
            .contributions
            .iter()
            .map(|(stratum, rank, value)| {
                format!(
                    "{} rank {rank} (+{})",
                    short(stratum),
                    value.to_decimal_lexical()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "  {}. {}  score {}  [{provenance}]",
            position + 1,
            row.entity.as_str(),
            row.score.to_decimal_lexical()
        );
    }

    println!("\nproducer statuses");
    for (stratum, status) in &result.trailer.statuses {
        let rendered = match status {
            ProducerStatus::Exhausted { rows_emitted } => {
                format!("exhausted after {rows_emitted} rows")
            }
            ProducerStatus::CeilingReached { bound } => {
                format!(
                    "stopped at the declared bound {}",
                    bound.to_decimal_lexical()
                )
            }
            ProducerStatus::ExecutionFailed { reason } => format!("could not run: {reason}"),
            ProducerStatus::TermsRejected => "declined the terms it was handed".to_owned(),
        };
        println!("  {}: {rendered}", short(stratum));
    }

    println!("\nrequest terms nothing served");
    if result.unserved_terms.is_empty() {
        println!("  (none)");
    } else {
        for unserved in &result.unserved_terms {
            println!("  term {}: {:?}", unserved.request_term, unserved.reason);
        }
    }

    println!("\nfused under profile {}", result.profile_id);
}

fn main() {
    let data = dataset();
    let registry = registry(&data);
    let statistics = NoStatistics;
    let environment = AdmissionEnvironment {
        registry: &registry,
        statistics: &statistics,
        fusion_profile: None,
    };
    let profile = profile();

    let result = block_on(search(
        &request(),
        &registry,
        &statistics,
        &*data,
        &environment,
        &profile,
        TOP_K,
    ))
    .expect("the host's producers answer");

    report(&result);
}
