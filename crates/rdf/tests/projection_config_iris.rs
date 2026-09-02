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
//! forgot it, would forget too. So this walks each profile's configuration document, finds
//! every string leaf that is currently a valid absolute IRI, and drives each one through three
//! states: the pristine value (accepted), a relative one, and a malformed one (both refused).
//! A new IRI-valued field added to any profile is covered the moment its configuration carries
//! it, with no edit here.
//!
//! ## The sweep is TOTAL over the profile list, by construction
//!
//! [`config_for`] is an exhaustive `match` over [`ProjectionProfile`], so a seventeenth profile
//! does not compile until it is given a configuration to sweep. That matters because nine of
//! the sixteen have a shipped JSON fixture and seven do not — the seven were silently
//! unswept while the header above claimed sixteen. Those seven are built here through the same
//! public constructors a caller uses and serialized with `ProjectionConfig::to_json`, so what
//! is swept is the real deserialization path in both cases.
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

use std::collections::BTreeMap;

use serde_json::Value;

use purrdf_rdf::{
    CsvwConfig, CsvwContext, CsvwMode, CsvwVocabulary, LpgConfig, LpgExecutionLimits, LpgScope,
    OboGraphsConfig, OboGraphsVocabulary, OboMetadataRoles, OboOwlRoles, OboRdfRoles,
    ProjectionConfig, ProjectionLimits, ProjectionProfile, SkosClassRoles, SkosConfig,
    SkosDocumentationRoles, SkosGraphSelection, SkosLabelRoles, SkosRelationRoles, SkosSourceRoles,
    SkosTargetRoles,
};

/// The vocabulary prefixes the constructed configurations name. Test-fixture IRIs only —
/// PurRDF mints no vocabulary of its own, so every one of these is caller-supplied here
/// exactly as it would be by a real caller.
const EX: &str = "https://example.org/";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const OBO: &str = "http://www.geneontology.org/formats/oboInOwl#";
const SKOS: &str = "http://www.w3.org/2004/02/skos/core#";

/// The nine profiles whose configuration ships as a JSON fixture.
///
/// These are swept as the exact bytes the repository commits, so the sweep sees what a real
/// `purrdf project --config` invocation would read rather than a re-serialization of it.
const FIXTURES: &[(ProjectionProfile, &str)] = &[
    (
        ProjectionProfile::Void,
        include_str!("fixtures/dataset-description/void.json"),
    ),
    (
        ProjectionProfile::DcatRdf,
        include_str!("fixtures/dataset-description/dcat-rdf.json"),
    ),
    (
        ProjectionProfile::CsvwTerms,
        include_str!("fixtures/csvw-terms.json"),
    ),
    (
        ProjectionProfile::OkfTerms,
        include_str!("fixtures/okf-terms.json"),
    ),
    (
        ProjectionProfile::Croissant11,
        include_str!("fixtures/research-objects/carrier/croissant-1.1.json"),
    ),
    (
        ProjectionProfile::DataCite46,
        include_str!("fixtures/research-objects/carrier/datacite-4.6.json"),
    ),
    (
        ProjectionProfile::Dcat3,
        include_str!("fixtures/research-objects/carrier/dcat-3.json"),
    ),
    (
        ProjectionProfile::FrictionlessDataPackage1,
        include_str!("fixtures/research-objects/carrier/frictionless-data-package-1.json"),
    ),
    (
        ProjectionProfile::RoCrate13,
        include_str!("fixtures/research-objects/carrier/ro-crate-1.3.json"),
    ),
];

/// The configuration DOCUMENT for `profile`, as the JSON bytes the deserializer reads.
///
/// EXHAUSTIVE over [`ProjectionProfile`] on purpose: a seventeenth profile fails to compile
/// here rather than joining the sweep silently unswept, which is precisely how the seven
/// constructed profiles below came to be omitted while this file claimed sixteen.
fn config_for(profile: ProjectionProfile) -> Vec<u8> {
    if let Some((_, text)) = FIXTURES.iter().find(|(named, _)| *named == profile) {
        return text.as_bytes().to_vec();
    }
    let config = match profile {
        ProjectionProfile::LpgCsv => ProjectionConfig::LpgCsv(lpg_config()),
        ProjectionProfile::Neo4jCsv => ProjectionConfig::Neo4jCsv(lpg_config()),
        ProjectionProfile::OpenCypher => ProjectionConfig::OpenCypher(lpg_config()),
        ProjectionProfile::Graphml => ProjectionConfig::Graphml(lpg_config()),
        ProjectionProfile::CsvwExact => ProjectionConfig::CsvwExact(csvw_config()),
        ProjectionProfile::OboGraphs => ProjectionConfig::OboGraphs(Box::new(obo_config())),
        ProjectionProfile::Skos => ProjectionConfig::Skos(Box::new(skos_config())),
        // Every remaining profile is covered by a shipped fixture, returned above. The arm is
        // written out rather than a wildcard so adding a profile is a compile error here.
        ProjectionProfile::CsvwTerms
        | ProjectionProfile::OkfTerms
        | ProjectionProfile::Croissant11
        | ProjectionProfile::RoCrate13
        | ProjectionProfile::DataCite46
        | ProjectionProfile::Dcat3
        | ProjectionProfile::DcatRdf
        | ProjectionProfile::Void
        | ProjectionProfile::FrictionlessDataPackage1 => {
            unreachable!("{profile} is a fixture profile and was returned above")
        }
    };
    config.to_json().expect("serialize the constructed config")
}

fn limits() -> ProjectionLimits {
    ProjectionLimits::new(64, 16_000_000, 64_000_000, 72_000_000, 16).expect("limits")
}

fn lpg_config() -> LpgConfig {
    LpgConfig::new(
        format!("{EX}type"),
        LpgScope::all(),
        limits(),
        LpgExecutionLimits::new(100_000, 100_000, 100_000, 100_000).expect("execution limits"),
    )
    .expect("LPG config")
}

fn csvw_config() -> CsvwConfig {
    CsvwConfig::new(
        format!("{EX}csvw-metadata"),
        CsvwContext::new(format!("{EX}csvw-context"), BTreeMap::default()).expect("CSVW context"),
        format!("{EX}csvw-group"),
        CsvwVocabulary::new("http://www.w3.org/ns/csvw#", RDF, RDFS, XSD).expect("CSVW vocabulary"),
        CsvwMode::Standard,
        limits(),
        20_000,
    )
    .expect("CSVW config")
}

fn obo_config() -> OboGraphsConfig {
    let rdf = OboRdfRoles::new(
        format!("{RDF}type"),
        format!("{RDF}reifies"),
        format!("{RDF}first"),
        format!("{RDF}rest"),
        format!("{RDF}nil"),
        format!("{XSD}string"),
        format!("{XSD}boolean"),
    )
    .expect("OBO RDF roles");
    let owl = OboOwlRoles::new(
        format!("{RDFS}label"),
        format!("{RDFS}comment"),
        format!("{RDFS}subClassOf"),
        format!("{RDFS}subPropertyOf"),
        format!("{RDFS}domain"),
        format!("{RDFS}range"),
        format!("{OWL}Ontology"),
        format!("{OWL}Class"),
        format!("{OWL}NamedIndividual"),
        format!("{OWL}ObjectProperty"),
        format!("{OWL}AnnotationProperty"),
        format!("{OWL}DatatypeProperty"),
        format!("{OWL}equivalentClass"),
        format!("{OWL}intersectionOf"),
        format!("{OWL}Restriction"),
        format!("{OWL}onProperty"),
        format!("{OWL}someValuesFrom"),
        format!("{OWL}allValuesFrom"),
        format!("{OWL}propertyChainAxiom"),
        format!("{OWL}deprecated"),
    )
    .expect("OBO OWL roles");
    let metadata = OboMetadataRoles::new(
        format!("{EX}definition"),
        format!("{OBO}hasExactSynonym"),
        format!("{OBO}hasBroadSynonym"),
        format!("{OBO}hasNarrowSynonym"),
        format!("{OBO}hasRelatedSynonym"),
        format!("{OBO}hasSynonymType"),
        format!("{OBO}hasDbXref"),
        format!("{OBO}inSubset"),
        format!("{OWL}versionInfo"),
    )
    .expect("OBO metadata roles");
    OboGraphsConfig::new(
        format!("{EX}ontology"),
        OboGraphsVocabulary::new(rdf, owl, metadata).expect("OBO vocabulary"),
        limits(),
        20_000,
    )
    .expect("OBO config")
}

fn skos_class_roles() -> SkosClassRoles {
    SkosClassRoles::new(
        format!("{RDF}type"),
        format!("{SKOS}Concept"),
        format!("{SKOS}ConceptScheme"),
    )
    .expect("SKOS classes")
}

fn skos_label_roles() -> SkosLabelRoles {
    SkosLabelRoles::new(
        format!("{SKOS}prefLabel"),
        format!("{SKOS}altLabel"),
        format!("{SKOS}hiddenLabel"),
        format!("{SKOS}notation"),
    )
    .expect("SKOS labels")
}

fn skos_documentation_roles() -> SkosDocumentationRoles {
    SkosDocumentationRoles::new(
        format!("{SKOS}note"),
        format!("{SKOS}changeNote"),
        format!("{SKOS}definition"),
        format!("{SKOS}editorialNote"),
        format!("{SKOS}example"),
        format!("{SKOS}historyNote"),
        format!("{SKOS}scopeNote"),
    )
    .expect("SKOS documentation")
}

fn skos_relation_roles() -> SkosRelationRoles {
    SkosRelationRoles::new(
        format!("{SKOS}broader"),
        format!("{SKOS}narrower"),
        format!("{SKOS}related"),
        format!("{SKOS}closeMatch"),
        format!("{SKOS}exactMatch"),
        format!("{SKOS}broadMatch"),
        format!("{SKOS}narrowMatch"),
        format!("{SKOS}relatedMatch"),
        format!("{SKOS}inScheme"),
        format!("{SKOS}hasTopConcept"),
        format!("{SKOS}topConceptOf"),
    )
    .expect("SKOS relations")
}

fn skos_config() -> SkosConfig {
    let roles = || {
        (
            skos_class_roles(),
            skos_label_roles(),
            skos_documentation_roles(),
            skos_relation_roles(),
        )
    };
    let (classes, labels, documentation, relations) = roles();
    let source =
        SkosSourceRoles::new(classes, labels, documentation, relations).expect("SKOS source roles");
    let (classes, labels, documentation, relations) = roles();
    let target =
        SkosTargetRoles::new(classes, labels, documentation, relations).expect("SKOS target roles");
    SkosConfig::new(
        source,
        target,
        format!("{EX}scheme"),
        SkosGraphSelection::DefaultGraph,
        limits(),
        20_000,
    )
    .expect("SKOS config")
}

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

/// EVERY profile the carrier declares, and every absolute-IRI-valued leaf of its
/// configuration document, is gated.
///
/// Three states per field: pristine (accepted), relative (refused), malformed (refused). And
/// a refusal must carry the workspace's shared `purrdf_iri` diagnostic code — the check that
/// keeps the projection layer from growing a private spelling of what it already shares with
/// every codec, the CLI and the bindings.
///
/// The per-profile leaf counts are pinned EXACTLY rather than as a floor. A floor is
/// satisfied by a configuration that lost a field, which is the direction that matters here:
/// a gate silently stops covering what it no longer sees. Moving a count is a one-line,
/// deliberate edit; discovering months later that a profile quietly shed its IRI fields is
/// not.
#[test]
fn every_iri_valued_configuration_field_is_gated_by_the_shared_layer() {
    // Exact, per profile. `ProjectionProfile::ALL` is the same closed list `config_for`
    // matches on, so a profile cannot be present in one and absent from the other.
    let expected: &[(ProjectionProfile, usize)] = &[
        (ProjectionProfile::LpgCsv, 1),
        (ProjectionProfile::Neo4jCsv, 1),
        (ProjectionProfile::OpenCypher, 1),
        (ProjectionProfile::Graphml, 1),
        (ProjectionProfile::CsvwExact, 7),
        (ProjectionProfile::CsvwTerms, 13),
        (ProjectionProfile::OkfTerms, 17),
        (ProjectionProfile::OboGraphs, 37),
        (ProjectionProfile::Skos, 51),
        (ProjectionProfile::Croissant11, 92),
        (ProjectionProfile::RoCrate13, 95),
        (ProjectionProfile::DataCite46, 58),
        (ProjectionProfile::Dcat3, 95),
        // ZERO, and correctly so: `dcat-rdf`'s only IRIs live inside its CONSTRUCT query
        // text, which is a query rather than an IRI field. Its own IRI field,
        // `document_base_iri`, is covered by the second test in this file — which is why the
        // zero is pinned here rather than treated as a profile nothing checks.
        (ProjectionProfile::DcatRdf, 0),
        (ProjectionProfile::Void, 41),
        (ProjectionProfile::FrictionlessDataPackage1, 54),
    ];

    let mut observed: Vec<(ProjectionProfile, usize)> = Vec::new();
    for profile in ProjectionProfile::ALL {
        let bytes = config_for(*profile);
        let pristine: Value = serde_json::from_slice(&bytes).expect("the configuration is JSON");
        assert!(
            refusal(&pristine).is_none(),
            "{profile}: the configuration must parse as-is"
        );

        let mut leaves = Vec::new();
        absolute_iri_leaves(&pristine, &mut Vec::new(), &mut leaves);
        observed.push((*profile, leaves.len()));

        for leaf in &leaves {
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
                // Unconditional. Gating the code check on the refusal's own wording made it
                // vacuous for exactly the refusals that had grown a private spelling — the
                // ones it existed to catch.
                assert!(
                    error.contains("iri-"),
                    "{profile}: `{}` refuses {bad:?} with a private spelling rather than the \
                     shared purrdf_iri code: {error}",
                    leaf.join(".")
                );
            }
        }
    }

    assert_eq!(
        observed, expected,
        "the per-profile IRI-field census moved. Every profile above is swept for every \
         absolute-IRI-valued leaf its configuration carries; if a field was added or removed \
         on purpose, update the count in the same edit"
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

/// `package_profile` is the ONE excluded leaf that must still be gated on its own terms —
/// and the registry-identifier form it is excluded FOR had no test at all.
///
/// The sweep above skips this leaf by name because Data Package v1 specifies `profile` as
/// "a URL **or** a registry identifier", so gating it as an IRI would make PurRDF unable to
/// emit the spec's own canonical value, `tabular-data-package`. That exclusion is a
/// judgement about what is VALID, and until now nothing executed the valid case: a
/// `grep -r tabular-data-package` over the tree returned nothing, so an IRI gate could have
/// been (re-)applied to this field and every test would still have passed while the
/// commonest real-world `datapackage.json` became unemittable.
///
/// So both sides run here. The registry identifier and the URL form are both accepted, and
/// the identities the field genuinely refuses are still refused — the exclusion is "not an
/// IRI", not "not checked".
#[test]
fn a_frictionless_package_profile_accepts_a_registry_identifier_and_a_url() {
    let pristine: Value = serde_json::from_str(include_str!(
        "fixtures/research-objects/carrier/frictionless-data-package-1.json"
    ))
    .expect("the fixture is JSON");

    // VALID — the two forms Data Package v1 names, including the registry identifier the
    // exclusion exists for.
    for good in [
        "tabular-data-package",
        "data-package",
        "fiscal-data-package",
        "https://example.org/profiles/data-package-v1",
    ] {
        let mut document = pristine.clone();
        document["config"]["package_profile"] = Value::String(good.to_owned());
        assert!(
            refusal(&document).is_none(),
            "package_profile must accept the registry identifier {good:?}: it is a \
             spec-valid Data Package v1 profile and refusing it would make a conformant \
             datapackage.json unemittable"
        );
    }

    // And the accepted value reaches the config verbatim, rather than being normalized
    // into something else on the way through.
    let mut document = pristine.clone();
    document["config"]["package_profile"] = Value::String("tabular-data-package".to_owned());
    let bytes = serde_json::to_vec(&document).expect("re-serialize");
    let config = ProjectionConfig::from_json(&bytes).expect("the registry identifier parses");
    let round_tripped = config.to_json().expect("re-serialize the parsed config");
    let seen: Value = serde_json::from_slice(&round_tripped).expect("valid JSON");
    assert_eq!(
        seen["config"]["package_profile"],
        Value::String("tabular-data-package".to_owned()),
        "the profile identity is carried verbatim"
    );

    // INVALID — the field is excluded from the IRI sweep, NOT from validation.
    for bad in ["", "has a space", "tabular\tdata\tpackage"] {
        let mut document = pristine.clone();
        document["config"]["package_profile"] = Value::String(bad.to_owned());
        let error =
            refusal(&document).unwrap_or_else(|| panic!("package_profile must refuse {bad:?}"));
        assert!(
            error.contains("profile"),
            "the refusal must name the field: {error}"
        );
    }
}
