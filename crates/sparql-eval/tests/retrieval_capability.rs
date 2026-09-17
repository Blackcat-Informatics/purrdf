// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ranked-retrieval capability declaration: it is owned, serializable data
//! (no function pointers), it is reported by `describe`, and it participates in
//! the registry's durable content fingerprint.

use std::sync::Arc;

use purrdf_sparql_eval::{
    BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor, PropertyFunction,
    PropertyFunctionRegistry, RankOrdering, RetrievalCapability, TermKind, TermPattern, Volatility,
};

const EX_REL: &str = "http://example.org/ns#ranked";
const EX_STRATUM: &str = "http://example.org/stratum/a";

/// A relation that declares whatever capability it was built with and never
/// actually opens — the declaration surface is the subject under test.
#[derive(Debug)]
struct CapabilityRelation {
    capability: RetrievalCapability,
    modes: [BindingPattern; 1],
}

impl CapabilityRelation {
    fn new(capability: RetrievalCapability) -> Self {
        Self {
            capability,
            modes: [PfArity::new(1, 1).all_free_mode()],
        }
    }
}

impl PropertyFunction for CapabilityRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        0
    }

    fn retrieval_capability(&self) -> RetrievalCapability {
        self.capability.clone()
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Err(EvalError::function(
            "the capability fixture is never invoked",
        ))
    }
}

fn ranked() -> RetrievalCapability {
    RetrievalCapability::Ranked {
        stratum: purrdf_core::parse_iri(EX_STRATUM).expect("fixture IRI"),
        accepted_terms: vec![TermPattern {
            kind: TermKind::Literal,
            datatype: None,
            language: Some("en".to_owned()),
            predicate: None,
        }],
        ordering: RankOrdering::StrictlyDescending,
        duplicates: DuplicatePolicy::Unique,
    }
}

fn registry_with(capability: RetrievalCapability) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(EX_REL, Arc::new(CapabilityRelation::new(capability)));
    registry
}

#[test]
fn describe_reports_the_declared_capability() {
    let registry = registry_with(ranked());
    let described = registry.describe().expect("declarations are readable");
    assert_eq!(described.len(), 1);
    assert_eq!(described[0].retrieval, ranked());
}

#[test]
fn not_ranked_is_an_explicit_declaration() {
    let registry = registry_with(RetrievalCapability::NotRanked);
    let described = registry.describe().expect("declarations are readable");
    assert_eq!(described[0].retrieval, RetrievalCapability::NotRanked);
    assert_ne!(
        RetrievalCapability::NotRanked.canonical_description(),
        ranked().canonical_description()
    );
}

#[test]
fn capability_participates_in_the_content_fingerprint() {
    let ranked_registry = registry_with(ranked());
    let plain_registry = registry_with(RetrievalCapability::NotRanked);
    assert_ne!(
        ranked_registry
            .content_fingerprint()
            .expect("declarations are readable"),
        plain_registry
            .content_fingerprint()
            .expect("declarations are readable"),
        "a producer's fusion participation is a declaration the fingerprint must cover"
    );
    // Two registries declaring the same ranked capability fingerprint alike.
    let same = registry_with(ranked());
    assert_eq!(
        ranked_registry
            .content_fingerprint()
            .expect("declarations are readable"),
        same.content_fingerprint()
            .expect("declarations are readable"),
    );
}

/// The grep gate on the declaration types: no function pointer may appear in the
/// module that defines them.
#[test]
fn declaration_types_contain_no_function_pointers() {
    const SOURCE: &str = include_str!("../src/property_fn.rs");
    assert!(
        !SOURCE.contains("fn("),
        "property_fn.rs contains a function pointer; capability declarations must be owned data"
    );
}
