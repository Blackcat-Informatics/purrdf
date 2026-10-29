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
