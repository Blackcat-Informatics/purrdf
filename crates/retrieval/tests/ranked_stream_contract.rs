// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The declaration-to-contract hop: what a producer stated where it was
//! registered is what the stream carries into the fusion engine.
//!
//! This is the seam an approximation disclosure has to survive intact. The
//! string a producer publishes is the string a consumer reads out of the
//! answer, so every hop between the two must move those bytes rather than a
//! summary, a normalization or a re-derivation of them. This file pins the
//! first hop; `fusion.rs` pins the rest of the route.

use std::sync::Arc;

use purrdf_retrieval::{
    CandidateDomains, Completeness, DomainTag, DuplicatePolicy, OrderFidelity, RankFidelity,
    StreamContract,
};
use purrdf_sparql_eval::{RankedDeclaration, TermKind, TermPattern};

/// Prose a producer might really publish, carrying the characters that the
/// canonical `<byte-len>:<bytes>` framing has to survive and that a delimiter
/// scheme would corrupt: a colon, a semicolon, the two record separators the
/// encoding uses internally, and a newline.
const EVIDENCE: &str =
    "approximate: beam search; recall unmeasured above 10^6 \u{1}see\u{6}\nthe oracle note";

const PERTURBATION: &str = "quantized: distances compared in 8-bit space; rank may improve";

fn declaration(fidelity: RankFidelity) -> RankedDeclaration {
    RankedDeclaration {
        stratum: purrdf_core::parse_iri("http://example.org/stratum/vector").expect("fixture IRI"),
        accepted_terms: Vec::new(),
        depth_placement: None,
        candidate_position: 0,
        duplicates: DuplicatePolicy::Unique,
        fidelity,
        domains: CandidateDomains::Unrestricted,
        block_position: None,
        mandatory: false,
    }
}

#[test]
fn declared_carries_the_completeness_evidence_byte_for_byte() {
    let contract = StreamContract::declared(&declaration(RankFidelity {
        completeness: Completeness::Lossy {
            evidence: Arc::from(EVIDENCE),
        },
        order: OrderFidelity::Faithful,
    }));

    let Completeness::Lossy { evidence } = &contract.fidelity.completeness else {
        panic!("the contract kept the declared loss");
    };
    assert_eq!(
        &**evidence, EVIDENCE,
        "the hop from declaration to contract moves the producer's own bytes: \
         no trimming, no normalization, no re-wording"
    );
}

#[test]
fn declared_carries_both_axes_independently() {
    let contract = StreamContract::declared(&declaration(RankFidelity {
        completeness: Completeness::Lossy {
            evidence: Arc::from(EVIDENCE),
        },
        order: OrderFidelity::Perturbed {
            evidence: Arc::from(PERTURBATION),
        },
    }));

    let collected: Vec<&str> = contract.fidelity.evidence().map(|e| &**e).collect();
    assert_eq!(
        collected,
        vec![EVIDENCE, PERTURBATION],
        "both disclosures survive, in axis order, and neither is merged into the other"
    );
    assert!(contract.fidelity.may_omit());
    assert!(contract.fidelity.order_is_unbounded());
}

#[test]
fn declared_carries_an_exhaustive_promise_unchanged() {
    // The neighbouring case, and the one every producer shipped before the term
    // existed. It must reach the contract as the top of the lattice, not as an
    // absence a consumer has to interpret.
    let contract = StreamContract::declared(&declaration(RankFidelity::EXACT));
    assert_eq!(contract.fidelity, RankFidelity::EXACT);
    assert!(!contract.fidelity.may_omit());
    assert!(!contract.fidelity.order_is_unbounded());
    assert_eq!(contract.fidelity.evidence().count(), 0);
}

#[test]
fn the_three_declared_terms_are_carried_side_by_side() {
    // Fidelity does not displace or alter the two promises that were already
    // here: a contract carries all three, and each reads back as declared.
    let tag = DomainTag::parse("http://example.org/domain/documents").expect("fixture tag");
    let mut decl = declaration(RankFidelity {
        completeness: Completeness::Lossy {
            evidence: Arc::from(EVIDENCE),
        },
        order: OrderFidelity::Faithful,
    });
    decl.duplicates = DuplicatePolicy::Allowed;
    decl.domains = CandidateDomains::within([tag.clone()]);

    let contract = StreamContract::declared(&decl);
    assert_eq!(contract.duplicates, DuplicatePolicy::Allowed);
    assert_eq!(contract.domains, CandidateDomains::within([tag]));
    assert!(contract.fidelity.may_omit());
}

#[test]
fn a_hand_built_contract_states_the_same_three_terms() {
    // The `new` path is what a caller assembling a stream outside the registry
    // uses, and it must be able to say everything `declared` can say -- or a
    // hand-built approximate producer would be unable to disclose itself.
    let contract = StreamContract::new(
        DuplicatePolicy::Unique,
        RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from(EVIDENCE),
            },
            order: OrderFidelity::Faithful,
        },
        CandidateDomains::Unrestricted,
    );
    assert_eq!(
        contract,
        StreamContract::declared(&declaration(RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from(EVIDENCE),
            },
            order: OrderFidelity::Faithful,
        })),
        "the two construction paths agree term for term, so a hand-built stream \
         and a registered one are read identically by the fusion engine"
    );
}

#[test]
fn an_evidence_string_is_shared_rather_than_copied_across_contracts() {
    // The evidence is an `Arc<str>` precisely so this hop is a refcount bump:
    // one producer serves many streams, and every stream's contract is cloned
    // into the fusion engine.
    let evidence: Arc<str> = Arc::from(EVIDENCE);
    let decl = declaration(RankFidelity {
        completeness: Completeness::Lossy {
            evidence: Arc::clone(&evidence),
        },
        order: OrderFidelity::Faithful,
    });

    let first = StreamContract::declared(&decl);
    let second = StreamContract::declared(&decl);
    let Completeness::Lossy { evidence: a } = &first.fidelity.completeness else {
        unreachable!()
    };
    let Completeness::Lossy { evidence: b } = &second.fidelity.completeness else {
        unreachable!()
    };
    assert!(
        Arc::ptr_eq(a, b),
        "two contracts from one declaration share the producer's string rather \
         than each allocating a copy of it"
    );
}

#[test]
fn a_term_pattern_does_not_leak_into_the_contract() {
    // Guards the shape of the hop: `StreamContract` carries the three promises
    // about ROWS and nothing about request matching, so adding a term to the
    // declaration must not widen what travels with the stream.
    let mut decl = declaration(RankFidelity::EXACT);
    decl.accepted_terms = vec![purrdf_sparql_eval::AcceptedTerm {
        pattern: TermPattern::of_kind(TermKind::Iri),
        placements: Vec::new(),
    }];
    let with_terms = StreamContract::declared(&decl);
    let without = StreamContract::declared(&declaration(RankFidelity::EXACT));
    assert_eq!(
        with_terms, without,
        "what a producer accepts is a matching concern, not a promise about its rows"
    );
}
