// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical term grammar separates tokens by `WS` (space, tab, carriage
//! return, line feed) and nothing else, observed through the public planner and
//! compiler rather than the private decoder.
//!
//! An entity seed is decoded when a producer's placement renders it. FORM FEED
//! was once skipped as a separator (through `is_ascii_whitespace`); it is now
//! refused, and every neighbour that uses one of the four `WS` characters still
//! compiles to a query carrying the seed as its constant.
//!
//! Fixtures use `example.org` throughout; every IRI is fixture configuration.

use std::sync::Arc;

use purrdf_retrieval::{
    AdmissionEnvironment, CompiledRetrieval, Iri, PlanError, RankFidelity, RequestTerm,
    RetrievalRequest, Statistics, Term, compile, plan,
};
use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, CandidateDomains, DuplicatePolicy, EvalError, ExclusionBasis,
    PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry, RankArithmetic,
    RankedDeclaration, RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
};

fn ex(suffix: &str) -> String {
    format!("http://example.org/{suffix}")
}

/// A producer that emits no rows: only the text it is compiled into is read.
struct Silent;

struct Empty;

impl PfCursor for Empty {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }
}

impl PropertyFunction for Silent {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        static MODES: std::sync::OnceLock<Vec<BindingPattern>> = std::sync::OnceLock::new();
        MODES.get_or_init(|| vec![BindingPattern::from_code("ff")])
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        10
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Ok(Box::new(Empty))
    }
}

struct Bounds;

impl Statistics for Bounds {
    fn source(&self) -> &'static str {
        "seed-whitespace"
    }

    fn revision(&self) -> &'static str {
        "r1"
    }

    fn cardinality(&self, predicate: &Iri) -> Option<u64> {
        (predicate.as_str() == ex("stratum/any")).then_some(5)
    }

    fn selectivity_ppm(&self, _subject: &Iri, _term: &RequestTerm) -> Option<u64> {
        None
    }
}

fn registry() -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(
        ex("pf/any"),
        Arc::new(Silent),
        RankedDeclaration {
            stratum: purrdf_core::parse_iri(&ex("stratum/any")).expect("fixture IRI"),
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern::of_kind(TermKind::Any),
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: 1,
                    datatype: None,
                }],
            }],
            depth_placement: None,
            candidate_position: 0,
            duplicates: DuplicatePolicy::Unique,
            fidelity: RankFidelity::EXACT,
            arithmetic: RankArithmetic::FloatFree,
            domains: CandidateDomains::Unrestricted,
            block_position: None,
            exclusion: ExclusionBasis::Unavailable,
            mandatory: false,
        },
    );
    registry
}

/// Plan and compile one seed, returning the compiled unit's text, or the
/// refusal's message from whichever public stage refused it.
fn compile_seed(text: &str) -> Result<String, String> {
    let registry = registry();
    let stats = Bounds;
    let request = RetrievalRequest::complete(vec![RequestTerm::EntitySeed {
        entity: Term::new(text),
    }]);
    let planned = plan(&request, &registry, &stats).map_err(|e: PlanError| e.to_string())?;
    let env = AdmissionEnvironment {
        registry: &registry,
        statistics: &stats,
        fusion_profile: None,
    };
    let compiled: CompiledRetrieval = compile(&planned, &env).map_err(|e| e.to_string())?;
    let mut units = compiled.units.into_iter();
    let unit = units.next().ok_or_else(|| "no unit".to_owned())?;
    Ok(unit.sparql())
}

#[test]
fn form_feed_is_refused_where_the_four_ws_characters_are_accepted() {
    // Every position a separator can occupy: around a lone term, and between
    // and around the components of a triple term.
    let shapes: [fn(&str) -> String; 4] = [
        |ws| format!("<http://example.org/s>{ws}"),
        |ws| {
            format!(
                "<<({ws}<http://example.org/s> <http://example.org/p> <http://example.org/o> )>>"
            )
        },
        |ws| {
            format!(
                "<<( <http://example.org/s>{ws}<http://example.org/p> <http://example.org/o> )>>"
            )
        },
        |ws| {
            format!(
                "<<( <http://example.org/s> <http://example.org/p> <http://example.org/o>{ws})>>"
            )
        },
    ];
    for shape in shapes {
        // The believed-invalid case: FORM FEED (and the other non-WS spaces).
        for bad in ["\u{c}", "\u{b}", "\u{a0}"] {
            let text = shape(bad);
            let refused = compile_seed(&text);
            assert!(
                refused.is_err(),
                "{text:?} must be refused, got {refused:?}"
            );
        }
        // The valid neighbours: each of the four WS characters, in the same spot.
        for good in [" ", "\t", "\r", "\n"] {
            let text = shape(good);
            let sparql = compile_seed(&text)
                .unwrap_or_else(|error| panic!("{text:?} must compile: {error}"));
            assert!(
                sparql.contains("http://example.org/s"),
                "the seed is the rendered constant: {sparql}"
            );
        }
    }
}

#[test]
fn the_decoded_seed_is_the_same_term_under_every_ws_separator() {
    let plain = compile_seed("<<( <http://example.org/s> <http://example.org/p> \"o\" )>>")
        .expect("the canonical spelling compiles");
    let tabbed = compile_seed("<<(\t<http://example.org/s>\r<http://example.org/p>\n\"o\"\t)>>")
        .expect("WS separators compile");
    assert_eq!(
        plain, tabbed,
        "WS spelling does not change the emitted text"
    );
}
