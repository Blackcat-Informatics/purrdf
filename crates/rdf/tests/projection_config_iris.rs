// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Every IRI-valued field of a projection configuration DOCUMENT is gated, at parse time,
//! by the shared `purrdf_iri` layer.
//!
//! `purrdf project --config FILE` / `purrdf lift --config FILE` deserialize one JSON document
//! into one profile's configuration type, and those types carry the IRIs the projection mints
//! into its output: `dataset_iri`, `generated_resource_base_iri`, `scheme_iri`,
//! `entity_base_iri`, `document_base_iri`, graph names, vocabulary role tables, predicates and
//! datatypes. A field that accepted a relative or malformed value would put a non-absolute IRI
//! into an emitted graph, which is the exact defect this work exists to delete.
//!
//! ## Why this is a sweep rather than a hand-written case per field
//!
//! There are dozens of such fields across sixteen profiles, and the failure mode that matters
//! is a field that was FORGOTTEN — which a hand-written list, written by the same person who
//! forgot it, would forget too. So this walks each shipped configuration fixture, finds every
//! string leaf that is currently a valid absolute IRI, and drives each one through three
//! states: the pristine value (accepted), a relative one, and a malformed one (both refused).
//! A new IRI-valued field added to any profile is covered the moment the fixture carries it,
//! with no edit here.
//!
//! ## The two string leaves that are deliberately NOT IRI-valued
//!
//! Both are excluded by name, with the reason, rather than silently skipped:
//!
//! * `package_profile` (Data Package v1) is `URL **or** registry identifier` by its own
//!   specification, so `tabular-data-package` is a correct value and refusing it would make
//!   PurRDF unable to emit a spec-valid `datapackage.json`. Its sibling `profile_iri` on the
//!   other four research-object profiles IS an IRI and IS gated.
//! * `context.value` is a JSON-LD `@context` BODY, copied byte-semantically into the emitted
//!   JSON. Its members are keywords and term definitions, not all of which are IRIs. The
//!   table that actually mints IRIs — `context.definitions` — is gated, one value at a time.

use serde_json::Value;

use purrdf_rdf::ProjectionConfig;

/// Every shipped projection configuration fixture, by the profile it configures.
const CONFIGS: &[(&str, &str)] = &[
    (
        "void",
        include_str!("fixtures/dataset-description/void.json"),
    ),
    (
        "dcat-rdf",
        include_str!("fixtures/dataset-description/dcat-rdf.json"),
    ),
    ("csvw-terms", include_str!("fixtures/csvw-terms.json")),
    ("okf-terms", include_str!("fixtures/okf-terms.json")),
    (
        "croissant-1.1",
        include_str!("fixtures/research-objects/carrier/croissant-1.1.json"),
    ),
    (
        "datacite-4.6",
        include_str!("fixtures/research-objects/carrier/datacite-4.6.json"),
    ),
    (
        "dcat-3",
        include_str!("fixtures/research-objects/carrier/dcat-3.json"),
    ),
    (
        "frictionless-data-package-1",
        include_str!("fixtures/research-objects/carrier/frictionless-data-package-1.json"),
    ),
    (
        "ro-crate-1.3",
        include_str!("fixtures/research-objects/carrier/ro-crate-1.3.json"),
    ),
];

/// A relative IRI reference, shaped so that it satisfies every non-IRI rule a field also has
/// (several bases must end in `/` or `#`). Only its lack of a scheme may be what refuses it.
const RELATIVE: &str = "relative-configuration-value/";

/// A malformed IRI: the scheme has a space in it. Same trailing `/` for the same reason.
const MALFORMED: &str = "ht tp://example.org/relative-configuration-value/";

/// The JSON pointer segments of every string leaf that is currently a valid absolute IRI.
fn absolute_iri_leaves(value: &Value, at: &mut Vec<String>, found: &mut Vec<Vec<String>>) {
    match value {
        Value::String(text) => {
            if purrdf_iri::BaseIri::parse(text).is_ok() && !is_excluded(at) {
                found.push(at.clone());
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                at.push(index.to_string());
                absolute_iri_leaves(item, at, found);
                at.pop();
            }
        }
        Value::Object(members) => {
            for (key, member) in members {
                at.push(key.clone());
                absolute_iri_leaves(member, at, found);
                at.pop();
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// Whether this pointer names one of the two deliberately non-IRI string leaves documented
/// in the module header.
fn is_excluded(at: &[String]) -> bool {
    if at.last().is_some_and(|key| key == "package_profile") {
        return true;
    }
    at.windows(2)
        .any(|pair| pair[0] == "context" && pair[1] == "value")
}

/// Replace the leaf at `at` with `replacement`.
fn set(document: &mut Value, at: &[String], replacement: &str) {
    let mut cursor = document;
    for (depth, key) in at.iter().enumerate() {
        cursor = match cursor {
            Value::Object(members) => members.get_mut(key).expect("the pointer was just walked"),
            Value::Array(items) => {
                let index: usize = key.parse().expect("array pointers are indices");
                items.get_mut(index).expect("the pointer was just walked")
            }
            other => panic!("pointer {at:?} runs into a leaf at depth {depth}: {other}"),
        };
    }
    *cursor = Value::String(replacement.to_owned());
}

/// The rendered failure of parsing `document`, or `None` when it was accepted.
fn refusal(document: &Value) -> Option<String> {
    let bytes = serde_json::to_vec(document).expect("re-serialize the mutated configuration");
    ProjectionConfig::from_json(&bytes)
        .err()
        .map(|error| error.to_string())
}

/// EVERY absolute-IRI-valued leaf of EVERY shipped configuration fixture is gated.
///
/// Three states per field: pristine (accepted), relative (refused), malformed (refused). And
/// a refusal that says "must be an absolute IRI" must carry the workspace's shared
/// `purrdf_iri` diagnostic code — the check that keeps the projection layer from growing a
/// private spelling of what it already shares with every codec, the CLI and the bindings.
#[test]
fn every_iri_valued_configuration_field_is_gated_by_the_shared_layer() {
    let mut swept = 0usize;
    for (profile, text) in CONFIGS {
        let pristine: Value = serde_json::from_str(text).expect("the fixture is JSON");
        assert!(
            refusal(&pristine).is_none(),
            "{profile}: the shipped fixture must parse as-is"
        );

        // A profile may legitimately contribute no leaf: the `dcat-rdf` fixture's only IRIs
        // live INSIDE its CONSTRUCT query text, which is a query rather than an IRI field.
        // Its own IRI field, `document_base_iri`, is covered by the test below.
        let mut leaves = Vec::new();
        absolute_iri_leaves(&pristine, &mut Vec::new(), &mut leaves);

        for leaf in &leaves {
            swept += 1;
            for bad in [RELATIVE, MALFORMED] {
                let mut mutated = pristine.clone();
                set(&mut mutated, leaf, bad);
                let error = refusal(&mutated).unwrap_or_else(|| {
                    panic!(
                        "{profile}: `{}` accepted the non-absolute value {bad:?}; a projection \
                         must never mint an IRI from one",
                        leaf.join(".")
                    )
                });
                if error.contains("must be an absolute IRI") {
                    assert!(
                        error.contains("iri-"),
                        "{profile}: `{}` refuses with a private spelling rather than the \
                         shared purrdf_iri code: {error}",
                        leaf.join(".")
                    );
                }
            }
        }
    }
    // A floor rather than an exact count: the sweep is meant to grow with the fixtures, and
    // a fixture that lost every IRI would otherwise pass vacuously.
    assert!(
        swept > 100,
        "only {swept} IRI-valued configuration fields were swept"
    );
}

/// `document_base_iri` is gated on the DESERIALIZATION path, not only in the builder.
///
/// This is the field the fixtures do not carry, and it was the live defect: `VoidConfig` and
/// `SkosConfig` route their raw mirror through `with_document_base_iri`, while
/// `DcatRdfConfig` derived `Deserialize` and so read the value straight into the field. A
/// configuration DOCUMENT could therefore name any string at all as the base the emitted DCAT
/// document is published at, and the only thing that noticed was the serializer, much later,
/// talking about a "serialization base IRI" rather than about the field the caller wrote.
#[test]
fn a_document_base_iri_is_gated_when_the_configuration_document_sets_it() {
    for (profile, text) in [
        (
            "dcat-rdf",
            include_str!("fixtures/dataset-description/dcat-rdf.json"),
        ),
        (
            "void",
            include_str!("fixtures/dataset-description/void.json"),
        ),
    ] {
        let pristine: Value = serde_json::from_str(text).expect("the fixture is JSON");

        // A valid ABSOLUTE base is accepted.
        let mut good = pristine.clone();
        good["config"]["document_base_iri"] =
            Value::String("http://example.org/doc.ttl".to_owned());
        assert!(
            refusal(&good).is_none(),
            "{profile}: an absolute document base must be accepted"
        );

        for bad in [RELATIVE, MALFORMED] {
            let mut mutated = pristine.clone();
            mutated["config"]["document_base_iri"] = Value::String(bad.to_owned());
            let error = refusal(&mutated)
                .unwrap_or_else(|| panic!("{profile}: document_base_iri accepted {bad:?}"));
            assert!(
                error.contains("document base IRI") && error.contains("iri-"),
                "{profile}: the refusal must name the field and carry the shared code: {error}"
            );
        }
    }
}
