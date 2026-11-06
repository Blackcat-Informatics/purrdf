// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Additional canonical vectors run by the existing actual packaged-WASM door.

use core::fmt::Write as _;
use purrdf_core::datatype::{XSD_BOOLEAN, XSD_INTEGER, XSD_NON_NEGATIVE_INTEGER};
use purrdf_entail::reasoner::{Reasoner, SchemaPreparationBudget, ServiceProof, Verdict};
use purrdf_iri::vocab::{owl, rdf, rdfs};
use std::sync::Arc;

use super::{EntailContext, FixtureError, FixtureInvariant, ReasoningError};

const PREFIX: &str = "schema-preparation-";

struct Case {
    name: &'static str,
    service: &'static str,
    argument: &'static str,
    input: String,
    answer: &'static str,
}

fn restrictions(filler: &str, data: bool, maximum: Option<u32>) -> String {
    let mut out = String::new();
    let qualifier = if data {
        owl::ON_DATA_RANGE
    } else {
        owl::ON_CLASS
    };
    for (label, predicate, count) in [
        (
            "minimum",
            owl::MIN_QUALIFIED_CARDINALITY,
            Some(if maximum.is_some() { 2 } else { 3 }),
        ),
        ("maximum", owl::MAX_QUALIFIED_CARDINALITY, maximum),
    ] {
        let Some(count) = count else { continue };
        writeln!(
            out,
            "<https://example.org/A> <{}> _:{label} .",
            rdfs::SUB_CLASS_OF
        )
        .unwrap();
        writeln!(
            out,
            "_:{label} <{}> <https://example.org/p> .",
            owl::ON_PROPERTY
        )
        .unwrap();
        writeln!(out, "_:{label} <{qualifier}> <{filler}> .").unwrap();
        writeln!(
            out,
            "_:{label} <{predicate}> \"{count}\"^^<{XSD_NON_NEGATIVE_INTEGER}> ."
        )
        .unwrap();
    }
    if data {
        writeln!(
            out,
            "<https://example.org/p> <{}> <{}> .",
            rdf::TYPE,
            owl::DATATYPE_PROPERTY
        )
        .unwrap();
    }
    out
}

fn cases() -> Vec<Case> {
    let object = restrictions("https://example.org/F", false, Some(1));
    let asserted = format!(
        "{object}<https://example.org/i> <{}> <https://example.org/A> .\n",
        rdf::TYPE
    );
    let largest = format!(
        "{}<https://example.org/i> <{}> <https://example.org/A> .\n",
        restrictions("https://example.org/F", false, Some(0)).replace(
            &format!("_:minimum <{}> \"2\"", owl::MIN_QUALIFIED_CARDINALITY),
            &format!(
                "_:minimum <{}> \"{}\"",
                owl::MIN_QUALIFIED_CARDINALITY,
                u32::MAX - 1
            ),
        ),
        rdf::TYPE
    );
    let derived = format!(
        "{object}<https://example.org/edge> <{}> <https://example.org/A> .\n<https://example.org/s> <https://example.org/edge> <https://example.org/i> .\n",
        rdfs::RANGE
    );
    let existential = format!(
        "{object}<https://example.org/B> <{}> _:exists .\n_:exists <{}> <https://example.org/edge> .\n_:exists <{}> <https://example.org/A> .\n<https://example.org/s> <{}> <https://example.org/B> .\n",
        rdfs::SUB_CLASS_OF,
        owl::ON_PROPERTY,
        owl::SOME_VALUES_FROM,
        rdf::TYPE
    );
    let boolean = format!(
        "{}<https://example.org/i> <{}> <https://example.org/A> .\n",
        restrictions(XSD_BOOLEAN, true, None),
        rdf::TYPE
    );
    let mut data = restrictions(XSD_INTEGER, true, Some(1));
    // The max qualifier is decimal, a genuinely different superset of integer.
    data = data.replace(
        &format!("_:maximum <{}> <{XSD_INTEGER}> .", owl::ON_DATA_RANGE),
        &format!(
            "_:maximum <{}> <{}> .",
            owl::ON_DATA_RANGE,
            purrdf_core::datatype::XSD_DECIMAL
        ),
    );
    writeln!(
        data,
        "<https://example.org/i> <{}> <https://example.org/A> .",
        rdf::TYPE
    )
    .unwrap();
    let union = format!(
        "{object}_:choice <{}> _:first .\n_:first <{}> <https://example.org/A> .\n_:first <{}> _:second .\n_:second <{}> <https://example.org/Good> .\n_:second <{}> <{}> .\n<https://example.org/i> <{}> _:choice .\n",
        owl::UNION_OF,
        rdf::FIRST,
        rdf::REST,
        rdf::FIRST,
        rdf::REST,
        rdf::NIL,
        rdf::TYPE
    );
    vec![
        Case {
            name: "schema-preparation-empty-consistency",
            service: "consistency",
            argument: "",
            input: object.clone(),
            answer: "consistency true\n",
        },
        Case {
            name: "schema-preparation-empty-class",
            service: "class-satisfiability",
            argument: "<https://example.org/A>",
            input: object,
            answer: "class-satisfiability false\nterm <https://example.org/A>\n",
        },
        Case {
            name: "schema-preparation-asserted",
            service: "consistency",
            argument: "",
            input: asserted,
            answer: "consistency false\n",
        },
        Case {
            name: "schema-preparation-largest-count",
            service: "consistency",
            argument: "",
            input: largest,
            answer: "consistency false\n",
        },
        Case {
            name: "schema-preparation-derived",
            service: "consistency",
            argument: "",
            input: derived,
            answer: "consistency false\n",
        },
        Case {
            name: "schema-preparation-existential",
            service: "consistency",
            argument: "",
            input: existential,
            answer: "consistency false\n",
        },
        Case {
            name: "schema-preparation-data-extent",
            service: "consistency",
            argument: "",
            input: boolean,
            answer: "consistency false\n",
        },
        Case {
            name: "schema-preparation-data-containment",
            service: "consistency",
            argument: "",
            input: data,
            answer: "consistency false\n",
        },
        Case {
            name: "schema-preparation-union-neighbor",
            service: "consistency",
            argument: "",
            input: union,
            answer: "consistency true\n",
        },
    ]
}

pub(super) fn check(
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<(), ReasoningError> {
    let committed = super::dl_proof_golden_vectors()?;
    for case in cases() {
        let vector = committed
            .iter()
            .find(|vector| vector.name() == case.name)
            .ok_or_else(|| FixtureError::SchemaMissing {
                name: case.name.to_owned(),
            })?;
        if vector.input() != case.input
            || vector.answer() != case.answer
            || vector.service() != case.service
            || vector.argument().trim_end() != case.argument
        {
            return Err(FixtureError::SchemaControl {
                name: case.name.to_owned(),
                expected: [
                    case.input,
                    case.answer.to_owned(),
                    case.service.to_owned(),
                    case.argument.to_owned(),
                ],
                produced: [
                    vector.input().to_owned(),
                    vector.answer().to_owned(),
                    vector.service().to_owned(),
                    vector.argument().trim_end().to_owned(),
                ],
            }
            .into());
        }
    }
    let produced = committed
        .iter()
        .filter(|vector| vector.name().starts_with(PREFIX))
        .count();
    let expected = cases().len();
    if produced != expected {
        return Err(FixtureError::SchemaInventory { expected, produced }.into());
    }
    controls(control)
}

fn controls(control: &mut purrdf_core::CanonicalizationControl<'_>) -> Result<(), ReasoningError> {
    let too_large = cases()
        .into_iter()
        .find(|case| case.name == "schema-preparation-largest-count")
        .expect("authored production boundary")
        .input
        .replace("4294967294", "4294967295");
    let refused_count = super::parse_premise(&too_large)?;
    let count_refusal = Reasoner::with_proofs(&refused_count, control).err();
    if !matches!(
        count_refusal.as_ref(),
        Some(purrdf_entail::EntailError::OwlSyntax(error))
            if matches!(error.as_ref(), purrdf_entail::OwlSyntaxError::CardinalityRange {
                value: u32::MAX, maximum, ..
            } if *maximum == u32::MAX - 1)
    ) {
        return Err(FixtureError::Invariant {
            condition: FixtureInvariant::CardinalityRefusal,
            actual: Some(too_large),
            cause: count_refusal
                .map(|cause| Box::new(ReasoningError::entail(EntailContext::Native, cause))),
        }
        .into());
    }
    let document = cases()
        .into_iter()
        .find(|case| case.name == "schema-preparation-derived")
        .expect("authored derived case")
        .input;
    let dataset = super::parse_premise(&document)?;
    for budget in [
        SchemaPreparationBudget {
            work: Some(0),
            bytes: None,
        },
        SchemaPreparationBudget {
            work: None,
            bytes: Some(0),
        },
    ] {
        let mut owner = Reasoner::with_proofs_and_preparation_budget(&dataset, budget, control)
            .map_err(|error| {
                ReasoningError::entail(
                    EntailContext::Operation("schema vector owner".into()),
                    error,
                )
            })?;
        let refused = owner.consistency();
        if *refused.answer() != Verdict::Unknown
            || refused.certificate().schema_obstruction().is_none()
        {
            return Err(FixtureError::Invariant {
                condition: FixtureInvariant::SchemaUnknown,
                actual: Some(super::render_dl_certificate(
                    "consistency",
                    refused.certificate(),
                )),
                cause: None,
            }
            .into());
        }
        let proof = refused.proof().ok_or_else(|| FixtureError::Invariant {
            condition: FixtureInvariant::SchemaReceipt,
            actual: Some(super::render_dl_certificate(
                "consistency",
                refused.certificate(),
            )),
            cause: None,
        })?;
        let decoded =
            ServiceProof::decode(&proof.encode()).map_err(|cause| ReasoningError::Proof {
                operation: None,
                cause: Box::new(cause),
            })?;
        if decoded.receipt() != proof.receipt() {
            return Err(FixtureError::SchemaOutput {
                condition: FixtureInvariant::SchemaReceiptIdentity,
                expected: super::render_dl_proof(proof),
                produced: super::render_dl_proof(&decoded),
            }
            .into());
        }
        let rendered = super::render_dl_certificate("consistency", refused.certificate());
        let read = super::parse_dl_certificate(&rendered)?;
        if &read != refused.certificate() {
            return Err(FixtureError::SchemaOutput {
                condition: FixtureInvariant::SchemaCertificate,
                expected: rendered,
                produced: super::render_dl_certificate("consistency", &read),
            }
            .into());
        }
        let checker = Reasoner::with_proofs_and_preparation_budget(&dataset, budget, control)
            .map_err(|error| {
                ReasoningError::entail(
                    EntailContext::Operation("schema vector checker".into()),
                    error,
                )
            })?;
        let context = checker
            .proof_context()
            .map_err(|cause| ReasoningError::entail(EntailContext::Native, cause))?;
        decoded
            .verify(
                &dataset,
                &purrdf_entail::Question::Consistency,
                Some(&read),
                &context,
                control,
            )
            .map_err(|cause| {
                ReasoningError::proof(
                    "typed portable refusal receipt failed independent replay",
                    cause,
                )
            })?;
        if decoded
            .verify(
                &dataset,
                &purrdf_entail::Question::Consistency,
                Some(&read.with_schema_obstruction(
                    purrdf_entail::reasoner::SchemaObstruction::Allocation,
                )),
                &context,
                control,
            )
            .is_ok()
        {
            return Err(FixtureError::Invariant {
                condition: FixtureInvariant::SchemaOriginalCause,
                actual: Some(super::render_dl_proof(&decoded)),
                cause: None,
            }
            .into());
        }
        owner
            .retry_schema_preparation(SchemaPreparationBudget::default())
            .map_err(|cause| ReasoningError::entail(EntailContext::Native, cause))?;
        let retried = owner.consistency();
        if *retried.answer() != Verdict::False {
            return Err(FixtureError::Invariant {
                condition: FixtureInvariant::SchemaRetry,
                actual: Some(super::render_dl_certificate(
                    "consistency",
                    retried.certificate(),
                )),
                cause: None,
            }
            .into());
        }
    }
    #[derive(Debug)]
    struct Stop;
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            true
        }
    }
    let stopped = Reasoner::with_stop(&dataset, Arc::new(Stop)).err();
    if !matches!(stopped.as_ref(), Some(purrdf_entail::EntailError::Stopped)) {
        return Err(FixtureError::Invariant {
            condition: FixtureInvariant::SchemaStop,
            actual: Some(document),
            cause: stopped
                .map(|cause| Box::new(ReasoningError::entail(EntailContext::Native, cause))),
        }
        .into());
    }
    Ok(())
}
