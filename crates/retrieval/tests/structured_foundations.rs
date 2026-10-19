// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed foundation owners used by retrieval and its native failures.

use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_testkit::error_capture::Capture;

#[derive(Debug)]
struct SpawnCause(u64);

impl core::fmt::Display for SpawnCause {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        panic!("inspecting the original spawn cause must not render it")
    }
}

impl std::error::Error for SpawnCause {}

impl StructuredError for SpawnCause {
    purrdf_hash::error_visit! {
        @tuple self (count) => "spawn-cause-control" {
            "count" => Unsigned(u128::from(*count)),
        }
    }
}

fn stack_refusals_borrow_original_native_causes_and_full_width_counts() {
    let error = purrdf_stack::StackError::Spawn(
        purrdf_hash::error::io_error(std::io::ErrorKind::Other, SpawnCause(u64::MAX)).into(),
    );
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "stack-spawn");
    let native = captured.child("cause").child("cause").child("cause");
    assert_eq!(native.code, "spawn-cause-control");
    assert!(
        matches!(native.field("count"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    let mut cause = std::error::Error::source(&error);
    loop {
        let current = cause.expect("the original native source must remain reachable");
        if let Some(original) = current.downcast_ref::<SpawnCause>() {
            assert_eq!(original.0, u64::MAX);
            break;
        }
        cause = current.source();
    }
    let floor = purrdf_stack::StackError::ExceedsFloor {
        requested: usize::MAX,
        available: 0,
    };
    let captured = Capture::read(&floor);
    assert!(
        matches!(captured.field("requested"), ErrorValue::ByteCount(value) if value == usize::MAX as u128)
    );
    assert!(matches!(
        captured.field("available"),
        ErrorValue::ByteCount(0)
    ));
}

fn scoped_bases_retain_the_original_iri_and_directive_provenance() {
    let input = String::from("https://example.org/original/base/");
    let base = purrdf_iri::ScopedBase::new(
        purrdf_iri::BaseIri::parse(&input).unwrap(),
        purrdf_iri::BaseOrigin::Directive {
            line: u64::MAX,
            column: u32::MAX,
        },
    );
    drop(input);
    let copied = base.clone();
    drop(base);
    let captured = Capture::read(&copied);
    assert!(matches!(
        captured.child("iri").field("iri"),
        ErrorValue::Iri("https://example.org/original/base/")
    ));
    let origin = captured.child("origin");
    assert!(
        matches!(origin.field("line"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(
        matches!(origin.field("column"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
    );
}

fn fixed_values_binding_masks_and_registry_ids_keep_their_exact_native_bits() {
    for raw in [i128::MIN, -1, 0, i128::MAX] {
        let fixed = purrdf_text::Fixed::from_raw(raw);
        let captured = Capture::read(&fixed);
        assert!(matches!(captured.field("raw"), ErrorValue::Signed(value) if value == raw));
        assert!(matches!(
            captured.field("scale_digits"),
            ErrorValue::Unsigned(12)
        ));
    }
    let pattern = purrdf_sparql_eval::BindingPattern::from_bound_positions(64, 0..64);
    let captured = Capture::read(&pattern);
    assert!(matches!(captured.field("arity"), ErrorValue::Unsigned(64)));
    assert!(
        matches!(captured.field("bound"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    let registry = purrdf_sparql_eval::RegistryId::from_raw(u64::MAX);
    let captured = Capture::read(&registry);
    assert!(
        matches!(captured.field("value"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(
        captured.field("scope"),
        ErrorValue::Code("process")
    ));
}

fn attestations_preserve_original_generation_and_caller_message_owners() {
    use purrdf_sparql_eval::{IndexGeneration, PfAttestation, ServiceLevel};

    let generation = String::from("original-generation-λ");
    let reason = String::from("caller-owned unavailable shard β");
    let attestation = PfAttestation {
        generation: IndexGeneration::declared(generation.clone()),
        service: ServiceLevel::incomplete(&reason),
    };
    drop((generation, reason));
    let copied = attestation.clone();
    drop(attestation);
    let captured = Capture::read(&copied);
    assert!(matches!(
        captured.child("generation").field("value"),
        ErrorValue::Text("original-generation-λ")
    ));
    assert!(matches!(
        captured.child("service").field("reason"),
        ErrorValue::Message("caller-owned unavailable shard β")
    ));
}

fn evaluator_conversion_retains_the_original_parser_cause_after_input_disposal() {
    // The native cause must not widen composed Result errors past Clippy's
    // existing large-error boundary, including on the portable target.
    const { assert!(size_of::<purrdf_retrieval::SearchError>() < 128) };
    let input = String::from("SELECT WHERE {");
    let cause = purrdf_sparql_algebra::SparqlParser::new()
        .parse_query(&input)
        .unwrap_err();
    drop(input);
    let expected = cause.clone();
    let failure = purrdf_sparql_eval::EvalError::from(cause);
    let copied = failure.clone();
    drop(failure);
    let source = std::error::Error::source(&copied).unwrap();
    assert_eq!(
        source.downcast_ref::<purrdf_sparql_algebra::ParseError>(),
        Some(&expected)
    );
    assert!(
        purrdf_sparql_algebra::SparqlParser::new()
            .parse_query("SELECT * WHERE {}")
            .is_ok()
    );
}

purrdf_testkit::harness_main!(
    evaluator_conversion_retains_the_original_parser_cause_after_input_disposal,
    stack_refusals_borrow_original_native_causes_and_full_width_counts,
    scoped_bases_retain_the_original_iri_and_directive_provenance,
    fixed_values_binding_masks_and_registry_ids_keep_their_exact_native_bits,
    attestations_preserve_original_generation_and_caller_message_owners,
);
