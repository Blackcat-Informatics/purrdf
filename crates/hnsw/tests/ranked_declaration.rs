// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The fourth governed channel of the approximation contract: what this index
//! declares to a consumer of COMPOSED rows.
//!
//! The other three channels all speak to a caller reading the relation
//! directly, and none survives composition — fuse these rows with an exhaustive
//! producer's and every stratum reports one terminal status, with nothing to say
//! which of them offered candidates and which returned everything it had. This
//! file pins what the declaration says, that it registers, that the evidence is
//! the profile's own bytes, and that reaching for the unranked registration by
//! mistake fails loudly rather than quietly.

use std::sync::Arc;

use purrdf_core::{DistanceMetric, IndexLossContract, TermValue};
use purrdf_hnsw::relation::{
    HnswRelation, HnswSpace, RankedHnswRegistration, composed_order_fidelity, order_fidelity,
    register_hnsw_relation,
};
use purrdf_hnsw::{HnswIndex, Params, VectorMatrix, profile};
use purrdf_sparql_eval::{
    CandidateDomains, Completeness, DuplicatePolicy, KnnGuard, OrderFidelity,
    PropertyFunctionRegistry, TermKind,
};

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const PREDICATE: &str = "https://example.org/pf#nearest";
const STRATUM: &str = "https://example.org/stratum/vector";

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("valid parameters")
}

/// A small deterministic space. The values are a splitmix walk mapped into
/// `(-1, 1)`; nothing here reads a clock or an RNG.
fn space_over(rows: usize, terms: Vec<TermValue>, params: Params) -> Arc<HnswSpace> {
    let dims = 4;
    let mut state = 0x51DE_0000_1234_ABCD_u64;
    let mut data = Vec::with_capacity(rows * dims);
    for _ in 0..rows * dims {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        let value = ((z >> 11) as f64 / (1u64 << 53) as f64).mul_add(2.0, -1.0);
        data.push(if value == 0.0 { 0.125 } else { value });
    }
    let matrix = VectorMatrix::new(rows, dims, data).expect("a valid matrix");
    let index =
        HnswIndex::build(matrix, &DistanceMetric::SquaredEuclidean, params).expect("it builds");
    let guard = KnnGuard::new(rows as u64, rows as u64).expect("a valid guard");
    Arc::new(HnswSpace::from_index(index, terms, guard).expect("a valid space"))
}

fn iris(rows: usize) -> Vec<TermValue> {
    (0..rows)
        .map(|row| TermValue::iri(format!("https://example.org/row/{row}")))
        .collect()
}

fn space(rows: usize) -> Arc<HnswSpace> {
    space_over(rows, iris(rows), params())
}

fn stratum() -> purrdf_core::Iri {
    purrdf_core::parse_iri(STRATUM).expect("fixture IRI")
}

fn declaration(space: &Arc<HnswSpace>) -> purrdf_sparql_eval::RankedDeclaration {
    declaration_with(space, OrderFidelity::Faithful)
}

/// The same declaration under a host that has something to disclose about the
/// vectors it handed the build.
///
/// `Faithful` is the "I did nothing to them" case above; it is spelled rather
/// than defaulted, because the top of an axis is the one direction a default
/// must never go.
fn declaration_with(
    space: &Arc<HnswSpace>,
    vector_order: OrderFidelity,
) -> purrdf_sparql_eval::RankedDeclaration {
    HnswRelation::new(Arc::clone(space)).ranked_declaration(
        stratum(),
        TermKind::Iri,
        XSD_INTEGER.to_owned(),
        vector_order,
        CandidateDomains::Unrestricted,
    )
}

// --- What the declaration says -------------------------------------------

#[test]
fn the_declaration_carries_the_profile_evidence_byte_for_byte() {
    let decl = declaration(&space(16));
    let Completeness::Lossy { evidence } = &decl.fidelity.completeness else {
        panic!("an HNSW search offers candidates and never certifies absence");
    };
    assert_eq!(
        &**evidence,
        profile::LOSS_EVIDENCE,
        "the string the producer publishes is the string a consumer reads: not \
         a boolean derived downstream, not a summary, not a re-wording"
    );
}

#[test]
fn the_declaration_names_the_positions_the_relation_really_renders() {
    let decl = declaration(&space(16));
    assert_eq!(decl.candidate_position, HnswRelation::NEIGHBOUR);
    assert_eq!(
        decl.accepted_terms[0].placements[0].position,
        HnswRelation::QUERY
    );
    assert_eq!(
        decl.depth_placement
            .as_ref()
            .expect("depth binds at `k`")
            .position,
        HnswRelation::COUNT
    );
    // Distinct, or one argument would render two values.
    assert_ne!(HnswRelation::NEIGHBOUR, HnswRelation::QUERY);
    assert_ne!(HnswRelation::QUERY, HnswRelation::COUNT);
    assert_ne!(HnswRelation::COUNT, HnswRelation::DISTANCE);
}

#[test]
fn the_declaration_promises_distinct_rows() {
    // The space enforces distinct terms at construction and one search visits a
    // node at most once, so this is a promise the relation can keep.
    assert_eq!(declaration(&space(16)).duplicates, DuplicatePolicy::Unique);
}

// --- The order axis is DERIVED, and both branches execute -----------------

#[test]
fn the_order_axis_is_a_function_of_the_loss_contract_and_not_a_literal() {
    let evidence: Arc<str> = Arc::from(profile::LOSS_EVIDENCE);

    // The shipped contract: a graph over untransformed vectors compares exact
    // distances for every candidate it visits, so the rows it returns are in
    // true relative order.
    let faithful = IndexLossContract {
        transforms_vectors: false,
        loss_encoding: None,
        loss_parameters: None,
    };
    assert_eq!(
        order_fidelity(&faithful, &evidence),
        OrderFidelity::Faithful
    );

    // The neighbouring contract, differing in exactly one field. Quantized
    // vectors mean approximated distances, so a row can be ranked BETTER than
    // it was due and no finite bound on the score error survives.
    let perturbed = IndexLossContract {
        transforms_vectors: true,
        loss_encoding: Some("int8".to_owned()),
        loss_parameters: Some(vec![8]),
    };
    assert_eq!(
        order_fidelity(&perturbed, &evidence),
        OrderFidelity::Perturbed {
            evidence: Arc::clone(&evidence)
        },
        "and it carries the same bytes: the axis changes, the disclosure does not"
    );
}

#[test]
fn the_shipped_relation_is_pinned_to_that_function() {
    // Not `assert_eq!(order, Faithful)`, which a hardcoded literal would also
    // satisfy. Pinning to the function means replacing the call with a literal
    // fails here.
    let space = space(16);
    let decl = declaration(&space);
    let evidence: Arc<str> = Arc::from(space.evidence());
    assert_eq!(
        decl.fidelity.order,
        composed_order_fidelity(
            order_fidelity(&profile::loss_contract(), &evidence),
            OrderFidelity::Faithful
        ),
    );
}

// --- The host's half of the order axis ------------------------------------

/// What the vectors were BEFORE the build is a fact the index cannot read, so
/// the host states it, and stating it degrades the axis the derivation left at
/// the top.
///
/// A host that product-quantizes its embeddings and then builds an HNSW graph
/// over the codes has an order-perturbed producer: the distances that ranked its
/// rows are approximations, so a row can arrive at a better rank than it was due
/// and no finite bound on a fused score survives. This build's loss contract
/// cannot see any of that — it describes what `HnswIndex::build` did, not what
/// reached it.
#[test]
fn a_host_that_approximated_its_vectors_declares_a_perturbed_order() {
    let host: Arc<str> = Arc::from(
        "the vectors handed to this build are int8 codes of the model's f32 \
         output; a distance between two codes approximates the distance the \
         caller meant",
    );
    let decl = declaration_with(
        &space(16),
        OrderFidelity::Perturbed {
            evidence: Arc::clone(&host),
        },
    );

    assert_eq!(
        decl.fidelity.order,
        OrderFidelity::Perturbed {
            evidence: Arc::clone(&host)
        },
        "the host's words, byte for byte, not a summary of them"
    );
    assert!(
        decl.fidelity.order_is_unbounded(),
        "which is the fact a consumer acts on: no finite score bound exists"
    );

    // And the profile's own disclosure is NOT displaced by the host's. It is
    // published on the completeness axis, which is lossy over every space on
    // every request, so it still reaches a consumer verbatim.
    let Completeness::Lossy { evidence } = &decl.fidelity.completeness else {
        panic!("an HNSW search offers candidates and never certifies absence");
    };
    assert_eq!(&**evidence, profile::LOSS_EVIDENCE);
}

/// The neighbour that must keep working: a host with nothing to disclose gets
/// exactly the declaration it got before, and the added parameter refuses
/// nothing.
#[test]
fn a_host_that_transformed_nothing_gets_the_declaration_it_always_had() {
    let space = space(16);
    let decl = declaration_with(&space, OrderFidelity::Faithful);
    assert_eq!(
        decl.fidelity.order,
        OrderFidelity::Faithful,
        "an untransformed pipeline over an untransforming profile is faithful \
         on both halves, so the meet of them is too"
    );
    let Completeness::Lossy { evidence } = &decl.fidelity.completeness else {
        panic!("the beam is still the beam");
    };
    assert_eq!(&**evidence, profile::LOSS_EVIDENCE);
}

/// The composition degrades and never upgrades, on every one of its four
/// inputs — including the branch the shipped profile cannot reach today.
///
/// The `Perturbed` × `Perturbed` corner is the one worth writing down: an axis
/// holds one disclosure, so a rule is needed, and the host's is the one that
/// wins. Nothing is lost by that, because the derived string is a second copy of
/// the space's evidence and the first copy is on the completeness axis, where
/// the approximation contract pins it.
#[test]
fn the_composition_takes_the_worse_of_the_two_and_keeps_the_hosts_words() {
    let derived: Arc<str> = Arc::from(profile::LOSS_EVIDENCE);
    let host: Arc<str> = Arc::from("https://example.org/disclosure/quantized-input");
    let host_perturbed = OrderFidelity::Perturbed {
        evidence: Arc::clone(&host),
    };
    let derived_perturbed = OrderFidelity::Perturbed {
        evidence: Arc::clone(&derived),
    };

    assert_eq!(
        composed_order_fidelity(OrderFidelity::Faithful, OrderFidelity::Faithful),
        OrderFidelity::Faithful
    );
    assert_eq!(
        composed_order_fidelity(derived_perturbed.clone(), OrderFidelity::Faithful),
        derived_perturbed,
        "a host saying `I did nothing` cannot talk a transforming profile back \
         up to the top of the axis"
    );
    assert_eq!(
        composed_order_fidelity(OrderFidelity::Faithful, host_perturbed.clone()),
        host_perturbed
    );
    assert_eq!(
        composed_order_fidelity(derived_perturbed, host_perturbed.clone()),
        host_perturbed,
        "and where both have something to say the host's is carried, because \
         the profile's is already on the completeness axis"
    );
}

// --- It registers, which is the neighbouring-valid case -------------------

#[test]
fn the_declaration_registers_through_the_ranked_path() {
    // The refusal added beside this term rejects a declared loss with empty
    // evidence. This is its neighbour, discharged on the real shipped producer:
    // a genuine disclosure must still register.
    let mut registry = PropertyFunctionRegistry::new();
    purrdf_hnsw::relation::register_ranked_hnsw_relation(
        &mut registry,
        PREDICATE,
        space(16),
        RankedHnswRegistration {
            stratum: stratum(),
            seed: TermKind::Iri,
            depth_datatype: XSD_INTEGER.to_owned(),
            vector_order: OrderFidelity::Faithful,
            domains: CandidateDomains::Unrestricted,
        },
    );

    let read_back = registry
        .ranked_declaration(PREDICATE)
        .expect("a ranked registration reads its declaration back");
    let Completeness::Lossy { evidence } = &read_back.fidelity.completeness else {
        panic!("the registry kept the declared loss");
    };
    assert_eq!(&**evidence, profile::LOSS_EVIDENCE);
}

#[test]
fn the_unranked_path_declares_nothing_and_so_forms_no_stratum() {
    // The reason `register_hnsw_relation` can safely remain. A host that
    // reaches for it by mistake does not get a stream that fuses while
    // claiming to be exhaustive -- it gets a relation with no ranked
    // declaration at all, which the planner reports as an unserved term rather
    // than composing.
    let mut registry = PropertyFunctionRegistry::new();
    register_hnsw_relation(&mut registry, PREDICATE, space(16));

    assert!(
        registry.resolve(PREDICATE).is_some(),
        "the relation is registered and answers SPARQL directly"
    );
    assert!(
        registry.ranked_declaration(PREDICATE).is_none(),
        "but it declares nothing to a consumer of composed rows, so it can \
         never contribute a stratum whose status would be read as complete"
    );
}

// --- The generation moves with the answers, and only with them ------------

#[test]
fn the_generation_moves_when_a_bound_term_moves() {
    let base = space(8);
    let mut altered_terms = iris(8);
    altered_terms[3] = TermValue::iri("https://example.org/row/moved");
    let altered = space_over(8, altered_terms, params());
    assert_ne!(
        base.generation(),
        altered.generation(),
        "two spaces over one graph bound to different terms return different \
         terms at position zero of every row, so they are different generations"
    );
}

#[test]
fn the_generation_moves_when_the_beam_moves() {
    // The contract distinction from the exact kNN space, made executable. That
    // space excludes its bound on work because the bound decides how hard a
    // search tries and never which rows exist. Here `ef_search` is never
    // widened to fit a request, so a narrower beam finds DIFFERENT rows -- a
    // parameter that changes the answer belongs in the identity of what
    // answered.
    let wide = space_over(16, iris(16), Params::new(4, 8, 16, 8).expect("valid"));
    let narrow = space_over(16, iris(16), Params::new(4, 8, 16, 4).expect("valid"));
    assert_ne!(wide.generation(), narrow.generation());
}

#[test]
fn the_generation_is_stable_across_identical_builds() {
    // The neighbour that must NOT move: a generation that changed when nothing
    // did would make every answer look freshly degraded.
    assert_eq!(space(12).generation(), space(12).generation());
}

#[test]
fn the_generation_is_declared_on_both_construction_paths() {
    // `from_artifact` delegates to `from_index`, so there is no path that
    // yields a space without one. A field present on one constructor and absent
    // on the other is the modal optionality this workspace refuses.
    let generation = space(8).generation().to_owned();
    assert_eq!(generation.len(), 64, "a lowercase hex content digest");
    assert!(
        generation
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
    );
}

// --- The beam is declared, so a status cannot lie about it ----------------

#[test]
fn the_declared_row_bound_never_exceeds_the_beam() {
    // `ef_search` is never widened to fit a request, so a `k` above it is
    // answered with fewer than `k` rows. A bound that ignored it would let a
    // plan ask for a depth the beam cannot reach, never see its limit met, and
    // be told the stream was exhausted -- a completeness-flavoured ending for a
    // read the parameters cut short.
    let narrow = space_over(64, iris(64), Params::new(4, 8, 16, 4).expect("valid"));
    let relation = HnswRelation::new(Arc::clone(&narrow));
    let mode = purrdf_core::binding_pattern::BindingPattern::from_code("fbbf");
    assert!(
        purrdf_sparql_eval::PropertyFunction::rows_per_invocation(&relation, mode)
            <= narrow.beam_width(),
        "the declared bound is the beam or less, never the guard's ceiling alone"
    );
}

#[test]
fn a_search_within_the_beam_is_still_served() {
    // The neighbour: declaring the beam must not refuse a request the index can
    // actually answer. A depth at or under the beam is served exactly as before.
    let narrow = space_over(64, iris(64), Params::new(4, 8, 16, 4).expect("valid"));
    let rows = narrow
        .index()
        .search_rows(0, 4)
        .expect("a search within the beam succeeds");
    assert!(!rows.is_empty(), "and it returns rows");
}
