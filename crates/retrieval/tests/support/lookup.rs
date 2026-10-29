// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The two-stratum lookup fixtures: the `left`/`right` strata, the profile they
//! fuse under, and the counting producer the exclusion-lookup tests ask.
//!
//! Fixtures use `example.org` throughout; every IRI below is fixture
//! configuration, never a minted vocabulary.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_retrieval::fixture::PRECISION_12;
use std::sync::{Arc, Mutex};

use purrdf_core::TermValue;
use purrdf_core::binding_pattern::BindingPattern;
use purrdf_retrieval::fixture::iri;
use purrdf_retrieval::{
    DecayRule, DomainTag, FusedRow, FusionProfile, Iri, ProducerStatus, RECIP_K, RequestTerm,
    RetrievalRequest, Score, TopK, contribution_under,
};
use purrdf_sparql_eval::{
    EvalError, IndexGeneration, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, ServiceLevel,
    Volatility,
};

/// How many rows each producer ranks.
pub const ROWS: u64 = 400;

/// The answer size every search here asks for.
pub const TOP_K: TopK = TopK::new(5);

/// The two request terms, one per stratum.
pub const PREDICATES: [&str; 2] = ["title", "body"];

/// Each stratum's candidates are minted under its own prefix, so no candidate is
/// held by both: every lookup of the other stratum's candidate is an exclusion.
pub const PREFIXES: [&str; 2] = ["left/", "right/"];

pub fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

/// The fixture block domain `http://example.org/{suffix}`.
pub fn domain_tag(suffix: &str) -> DomainTag {
    DomainTag::parse(&ex(suffix)).expect("fixture domain tags are valid IRIs")
}

pub fn producer_iri(predicate: &str) -> String {
    ex(&format!("pf/{predicate}"))
}

pub fn strata() -> [Iri; 2] {
    [iri(&ex("stratum/left")), iri(&ex("stratum/right"))]
}

/// Both strata at weight one, under reciprocal-rank decay with the library's
/// smoothing constant.
pub fn profile() -> FusionProfile {
    FusionProfile::with_decay(
        strata()
            .into_iter()
            .map(|stratum| (stratum, PRECISION_12.score(PRECISION_12.scale())))
            .collect(),
        DecayRule::ReciprocalRank {
            k: u32::try_from(RECIP_K).expect("the smoothing constant fits"),
        },
        PRECISION_12,
    )
    .expect("the fixture profile is valid")
}

/// A stopped live stream's ceiling under the fixture's weight-one rank law.
#[must_use]
pub fn ceiling_at(rank: u64) -> ProducerStatus {
    ProducerStatus::CeilingReached {
        bound: contribution_under(
            DecayRule::ReciprocalRank { k: RECIP_K as u32 },
            PRECISION_12.score(PRECISION_12.scale()),
            rank,
        )
        .expect("the fixture contribution is in range"),
    }
}

/// A producer ranking `{prefix}entity{index:06}` for every index below [`ROWS`],
/// whose candidate-bound call answers from the candidate IRI's own prefix.
///
/// Every invocation's mode is recorded, so a test reads exactly how many lookups
/// the search asked and in which mode, from the producer's side.
pub struct Producer {
    pub arity: PfArity,
    pub modes: Vec<BindingPattern>,
    pub prefix: &'static str,
    pub asked: Arc<Mutex<Vec<String>>>,
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

    fn open<'cursor>(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor + 'cursor>, EvalError>
    where
        Self: 'cursor,
    {
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

pub struct Rows(pub std::vec::IntoIter<PfRow>);

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

pub fn request() -> RetrievalRequest {
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

pub fn asked(pairs: &[(&str, usize)]) -> Vec<(String, usize)> {
    pairs
        .iter()
        .map(|(code, count)| ((*code).to_owned(), *count))
        .collect()
}

/// The five entities every search here answers, as the fusion spells a term:
/// both strata's first three, fused under equal weights, cut to five.
pub fn expected_entities() -> Vec<String> {
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

/// A fused answer as `(entity, score)` pairs, in rank order.
pub fn scored(rows: &[FusedRow]) -> Vec<(String, Score)> {
    rows.iter()
        .map(|row| (row.entity.as_str().to_owned(), row.score))
        .collect()
}
