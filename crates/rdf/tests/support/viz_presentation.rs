// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generic caller-supplied component and scenario examples.

use purrdf_rdf::TermValue;
use purrdf_rdf::viz::{
    VizComponent, VizDisplay, VizGraphInput, VizInputQuad, VizInputReifier, VizInputStatement,
    VizInteraction, VizNavigation, VizParticipant, VizPresentation, VizProjection, VizRegionKind,
    VizScenario, VizScenarioRegion, VizSceneLabel, VizSemanticRef, VizTermId, VizTermValue,
    VizTextDirection, VizValueRef,
};

const EX: &str = "https://example.org/";

#[cfg(test)]
pub(crate) fn boundary_return_architecture(model: &VizProjection) -> VizPresentation {
    let declared = [
        (
            "component/delivery",
            "Delivery service",
            "DECLARED responsibility: Own parcel delivery",
            "declared boundary",
        ),
        (
            "component/router",
            "Router",
            "DECLARED responsibility: Validate and route parcels",
            "declared component",
        ),
        (
            "component/storage",
            "Storage",
            "DECLARED responsibility: Retain accepted parcels",
            "declared component",
        ),
        (
            "actor/customer",
            "Customer",
            "Submit parcels and receive decisions",
            "declared actor",
        ),
        (
            "artifact/parcel",
            "Submitted parcel",
            "declared type: Parcel",
            "declared artifact",
        ),
        (
            "artifact/stored-parcel",
            "Accepted parcel",
            "declared type: Parcel",
            "declared artifact",
        ),
        (
            "relation/submit",
            "Submit parcel",
            "declared protocol: HTTPS",
            "declared relation",
        ),
        (
            "relation/persist",
            "Store accepted parcel",
            "declared protocol: Repository interface",
            "declared relation",
        ),
        (
            "relation/respond",
            "Report decision",
            "declared protocol: HTTPS response",
            "declared relation",
        ),
    ];
    let binding = |suffix: &str| {
        VizSemanticRef::Term(model.terms.iter().find(|term| matches!(&term.value, VizTermValue::Iri { value } if value.ends_with(suffix))).expect("fixture term").id.clone())
    };
    let mut displays: Vec<_> = declared
        .into_iter()
        .map(|(suffix, label, description, role)| VizDisplay {
            binding: binding(suffix),
            label: Some(VizSceneLabel::plain(label)),
            description: Some(description.to_owned()),
            role: Some(role.to_owned()),
            navigation: None,
        })
        .collect();
    for (suffix, label, location, navigation) in [
        (
            "source/cbe55032fdc9015e97b6de27da947c457da69fb82f47327c3fe6c51798c175df",
            "parcel::Parcel",
            "src/lib.rs:1",
            "../../../../src/lib.rs#L1",
        ),
        (
            "source/dd79bfa63bf6059f2616e893d1922128ebd817785c870468156986d6fd3c111f",
            "parcel::route",
            "src/lib.rs:2",
            "../../../../src/lib.rs#L2",
        ),
        (
            "source/9e42811eb0e212a52a5e61d9f4f8d345739d322ddae3d42aac236fb575f5ac4b",
            "parcel::store",
            "src/lib.rs:3",
            "../../../../src/lib.rs#L3",
        ),
        (
            "source/95276124546e7434336bfebba52d9ff0a60fde8aec3f7d81a3bca94f7733de89",
            "parcel::test_crate::flow::case",
            "tests/flow.rs:2",
            "../../../../tests/flow.rs#L2",
        ),
    ] {
        displays.push(VizDisplay {
            binding: binding(suffix),
            label: Some(VizSceneLabel::plain(label)),
            description: Some(format!(
                "resolved identity at {location}; no execution claim"
            )),
            role: Some("source binding".to_owned()),
            navigation: Some(VizNavigation::parse(navigation).expect("fixture navigation")),
        });
    }
    let components = [
        ("delivery", None, 0),
        ("router", Some("delivery"), 1),
        ("storage", Some("delivery"), 2),
    ]
    .into_iter()
    .map(|(id, parent, index)| VizComponent {
        id: id.to_owned(),
        binding: displays[index].binding.clone(),
        parent: parent.map(str::to_owned),
        members: Vec::new(),
        label: displays[index].label.clone().expect("component label"),
    })
    .collect();
    VizPresentation {
        displays,
        components,
        scenario: None,
    }
}

pub(crate) fn input() -> VizGraphInput {
    let quad = |from: &str, predicate: &str, to: &str| VizInputQuad {
        subject: TermValue::Iri(format!("{EX}{from}")),
        predicate: format!("{EX}{predicate}"),
        object: TermValue::Iri(format!("{EX}{to}")),
        graph_name: Some(TermValue::Iri(format!("{EX}facts"))),
    };
    VizGraphInput {
        quads: vec![
            quad("system", "calls", "app"),
            quad("app", "queries", "database"),
            quad("app", "updates", "database"),
            quad("app", "caches", "app"),
            quad("app", "refreshes", "app"),
        ],
        reifiers: vec![VizInputReifier {
            reifier: TermValue::Iri(format!("{EX}cache-claim")),
            statement: VizInputStatement {
                subject: TermValue::Iri(format!("{EX}app")),
                predicate: format!("{EX}caches"),
                object: TermValue::Iri(format!("{EX}app")),
            },
            graph_name: Some(TermValue::Iri(format!("{EX}facts"))),
        }],
        annotations: Vec::new(),
    }
}

pub(crate) fn term(model: &VizProjection, local: &str) -> VizTermId {
    model
        .terms
        .iter()
        .find(
            |t| matches!(&t.value, VizTermValue::Iri { value } if value == &format!("{EX}{local}")),
        )
        .expect("example term")
        .id
        .clone()
}

pub(crate) fn architecture(model: &VizProjection) -> VizPresentation {
    VizPresentation {
        displays: vec![VizDisplay {
            binding: VizSemanticRef::Term(term(model, "app")),
            label: Some(VizSceneLabel::plain(
                "Account service with a deliberately long display label",
            )),
            description: Some(
                "Handles an explicit request; RDF identities remain inspectable.".to_owned(),
            ),
            role: Some("component".to_owned()),
            navigation: Some(
                VizNavigation::parse("../reference/service.html#api").expect("safe local link"),
            ),
        }],
        components: vec![
            VizComponent {
                id: "system".to_owned(),
                binding: VizSemanticRef::Term(term(model, "system")),
                parent: None,
                members: Vec::new(),
                label: VizSceneLabel::plain(
                    "Application boundary for independently named service components and cross-boundary interactions",
                ),
            },
            VizComponent {
                id: "service".to_owned(),
                binding: VizSemanticRef::Term(term(model, "app")),
                parent: Some("system".to_owned()),
                members: Vec::new(),
                label: VizSceneLabel {
                    text: "خدمة الحساب".to_owned(),
                    full_text: "خدمة الحساب".to_owned(),
                    language: Some("ar".to_owned()),
                    direction: Some(VizTextDirection::Rtl),
                },
            },
        ],
        scenario: None,
    }
}

pub(crate) fn scenario(model: &VizProjection) -> VizScenario {
    let interaction = |id: &str, predicate: &str, region: Option<&str>, order, label: &str| {
        let statement = model.statements.iter().find(|s| model.terms.iter().any(|t| t.id == s.predicate && matches!(&t.value, VizTermValue::Iri { value } if value == &format!("{EX}{predicate}")))).expect("example statement");
        let (VizValueRef::Term { id: from }, VizValueRef::Term { id: to }) =
            (&statement.subject, &statement.object)
        else {
            panic!("ordinary example endpoints")
        };
        VizInteraction {
            id: id.to_owned(),
            binding: VizSemanticRef::Statement(statement.id.clone()),
            from: from.clone(),
            to: to.clone(),
            region: region.map(str::to_owned),
            order,
            label: Some(VizSceneLabel::plain(label)),
        }
    };
    let region = |id: &str, parent: Option<&str>, order, kind, label: &str| VizScenarioRegion {
        id: id.to_owned(),
        parent: parent.map(str::to_owned),
        order,
        kind,
        label: VizSceneLabel::plain(label),
    };
    VizScenario {
        participants: [
            ("system", "Requester", "actor"),
            ("app", "Account service", "service"),
            ("database", "Record store", "store"),
        ]
        .into_iter()
        .map(|(id, name, role)| VizParticipant {
            term: term(model, id),
            label: VizSceneLabel::plain(name),
            role: Some(role.to_owned()),
        })
        .collect(),
        regions: vec![
            region(
                "result",
                None,
                1,
                VizRegionKind::Alternative,
                "Query result",
            ),
            region(
                "found",
                Some("result"),
                0,
                VizRegionKind::Branch,
                "record exists",
            ),
            region(
                "failure",
                Some("result"),
                1,
                VizRegionKind::Error,
                "query fails",
            ),
            region(
                "concurrent",
                None,
                2,
                VizRegionKind::Parallel,
                "Independent effects",
            ),
            region(
                "store",
                Some("concurrent"),
                0,
                VizRegionKind::Lane,
                "store request",
            ),
            region(
                "cache",
                Some("concurrent"),
                1,
                VizRegionKind::Lane,
                "local cache",
            ),
        ],
        interactions: vec![
            interaction("request", "calls", None, 0, "Read account"),
            interaction("read", "queries", Some("found"), 0, "Read matching record"),
            interaction(
                "refuse",
                "queries",
                Some("failure"),
                0,
                "Report query failure",
            ),
            interaction("write", "queries", Some("store"), 0, "Store independently"),
            interaction(
                "cache-update",
                "caches",
                Some("cache"),
                0,
                "Update local cache",
            ),
        ],
    }
}
