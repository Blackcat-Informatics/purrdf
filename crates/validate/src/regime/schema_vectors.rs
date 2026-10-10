// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Additional canonical vectors run by the existing actual packaged-WASM door.

use core::fmt::Write as _;
use purrdf_core::datatype::{XSD_BOOLEAN, XSD_INTEGER, XSD_NON_NEGATIVE_INTEGER};
use purrdf_entail::reasoner::{Reasoner, SchemaPreparationBudget, ServiceProof, Verdict};
use purrdf_iri::vocab::{owl, rdf, rdfs};
use std::sync::Arc;

use super::INPUT_MEDIA_TYPE;

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

pub(super) fn check() -> Result<(), String> {
    let committed = super::dl_proof_golden_vectors()?;
    for case in cases() {
        let vector = committed
            .iter()
            .find(|vector| vector.name() == case.name)
            .ok_or_else(|| {
                format!(
                    "missing canonical native/portable schema vector {}",
                    case.name
                )
            })?;
        if vector.input() != case.input
            || vector.answer() != case.answer
            || vector.service() != case.service
            || vector.argument().trim_end() != case.argument
        {
            return Err(format!(
                "schema vector {} no longer carries the actual authored control",
                case.name
            ));
        }
    }
    if committed
        .iter()
        .filter(|vector| vector.name().starts_with(PREFIX))
        .count()
        != cases().len()
    {
        return Err("schema preparation vector inventory mismatch".to_owned());
    }
    controls()
}

fn controls() -> Result<(), String> {
    let too_large = cases()
        .into_iter()
        .find(|case| case.name == "schema-preparation-largest-count")
        .expect("authored production boundary")
        .input
        .replace("4294967294", "4294967295");
    let refused_count = purrdf_rdf::parse_dataset(too_large.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|error| error.to_string())?;
    if !matches!(
        Reasoner::with_proofs(&refused_count),
        Err(purrdf_entail::EntailError::Parse(detail))
            if detail.contains("4294967295") && detail.contains("4294967294")
    ) {
        return Err(
            "portable boundary did not preserve the typed unrepresentable-count refusal".to_owned(),
        );
    }
    let document = cases()
        .into_iter()
        .find(|case| case.name == "schema-preparation-derived")
        .expect("authored derived case")
        .input;
    let dataset = purrdf_rdf::parse_dataset(document.as_bytes(), INPUT_MEDIA_TYPE, None)
        .map_err(|error| error.to_string())?;
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
        let mut owner = Reasoner::with_proofs_and_preparation_budget(&dataset, budget)
            .map_err(|error| error.to_string())?;
        let refused = owner.consistency();
        if *refused.answer() != Verdict::Unknown
            || refused.certificate().schema_obstruction().is_none()
        {
            return Err("native schema refusal was reported as a decided answer".to_owned());
        }
        let proof = refused
            .proof()
            .ok_or("refused recorded service has no receipt")?;
        let decoded = ServiceProof::decode(&proof.encode()).map_err(|error| error.to_string())?;
        if decoded.receipt() != proof.receipt() {
            return Err("typed portable refusal receipt changed".to_owned());
        }
        let rendered = super::render_dl_certificate("consistency", refused.certificate());
        let read = super::parse_dl_certificate(&rendered)?;
        if &read != refused.certificate() {
            return Err("portable certificate lost its preparation obstruction".to_owned());
        }
        let checker = Reasoner::with_proofs_and_preparation_budget(&dataset, budget)
            .map_err(|error| error.to_string())?;
        let context = checker.proof_context().map_err(|error| error.to_string())?;
        decoded
            .verify(
                &dataset,
                &purrdf_entail::Question::Consistency,
                Some(&read),
                &context,
            )
            .map_err(|error| {
                format!("typed portable refusal receipt failed independent replay: {error}")
            })?;
        if decoded
            .verify(
                &dataset,
                &purrdf_entail::Question::Consistency,
                Some(&read.with_schema_obstruction(
                    purrdf_entail::reasoner::SchemaObstruction::Allocation,
                )),
                &context,
            )
            .is_ok()
        {
            return Err("portable refusal accepted a different original cause".to_owned());
        }
        owner
            .retry_schema_preparation(SchemaPreparationBudget::default())
            .map_err(|error| error.to_string())?;
        if *owner.consistency().answer() != Verdict::False {
            return Err("sufficient native retry did not complete the same source".to_owned());
        }
    }
    #[derive(Debug)]
    struct Stop;
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            true
        }
    }
    if !matches!(
        Reasoner::with_stop(&dataset, Arc::new(Stop)),
        Err(purrdf_entail::EntailError::Stopped)
    ) {
        return Err("native schema construction ignored a latched stop".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{check_dl_proof, prove_to_string};
    use super::*;

    /// Seal only the new vectors from real native production output. Existing
    /// historical vectors are retained byte for byte, never regenerated here.
    #[test]
    #[ignore = "one-time native sealing after independent semantic/owner checks"]
    fn seal_schema_vectors() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/dl-proof.vectors");
        let old = std::fs::read_to_string(&path).unwrap();
        let mut out = old
            .split("@case schema-preparation-")
            .next()
            .unwrap()
            .to_owned();
        while !out.ends_with("\n\n") {
            out.push('\n');
        }
        for (index, case) in cases().into_iter().enumerate() {
            if index != 0 {
                out.push('\n');
            }
            let produced = prove_to_string(&case.input, case.service, case.argument, 0, 0).unwrap();
            assert_eq!(produced.answer(), case.answer, "{}", case.name);
            let checked = check_dl_proof(
                &case.input,
                case.service,
                case.argument,
                produced.answer(),
                produced.certificate(),
                produced.proof_document(),
            )
            .unwrap();
            assert!(
                checked.contains("unattested 0\n"),
                "{}: {checked}",
                case.name
            );
            writeln!(out, "@case {}\n@service {}\n@argument\n{}\n@input\n{}@answer\n{}@proof\n{}@check\n{}@end",
                case.name, case.service, case.argument, case.input, produced.answer(), produced.proof_document(), checked).unwrap();
        }
        controls().unwrap();
        std::fs::write(path, out).unwrap();
    }
}
