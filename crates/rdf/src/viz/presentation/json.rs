// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native closed record codecs for the typed presentation layer.

use super::{
    VizBoundaryPort, VizComponent, VizDisplay, VizInteraction, VizInteractionLayout, VizNavigation,
    VizParticipant, VizParticipantLayout, VizPresentation, VizPresentationFrame,
    VizPresentationLayout, VizPresentedExport, VizPresentedScene, VizRegionKind, VizScenario,
    VizScenarioRegion,
};
use purrdf_lex::json::Value;
use purrdf_lex::json::record::{DecodeError, FromJson, ToJson};

impl ToJson for VizNavigation {
    fn to_json(&self) -> Value {
        self.as_str().to_json()
    }
}

impl FromJson for VizNavigation {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let text = String::from_json(value)?;
        Self::parse(&text).map_err(DecodeError::custom)
    }
}

purrdf_lex::json_string_enum!(VizRegionKind {
    Sequence => "sequence", Alternative => "alternative", Branch => "branch",
    Error => "error", Parallel => "parallel", Lane => "lane",
});

purrdf_lex::json_record!(VizDisplay as "visualization display" {
    "binding" => binding: required,
    "label" => label: optional,
    "description" => description: optional,
    "role" => role: optional,
    "navigation" => navigation: optional,
});
purrdf_lex::json_record!(VizComponent as "visualization component" {
    "id" => id: required,
    "binding" => binding: required,
    "parent" => parent: optional,
    "members" => members: required,
    "label" => label: required,
});
purrdf_lex::json_record!(VizParticipant as "scenario participant" {
    "term" => term: required,
    "label" => label: required,
    "role" => role: optional,
});
purrdf_lex::json_record!(VizScenarioRegion as "scenario region" {
    "id" => id: required,
    "parent" => parent: optional,
    "order" => order: required,
    "kind" => kind: required,
    "label" => label: required,
});
purrdf_lex::json_record!(VizInteraction as "scenario interaction" {
    "id" => id: required,
    "binding" => binding: required,
    "from" => from: required,
    "to" => to: required,
    "region" => region: optional,
    "order" => order: required,
    "label" => label: optional,
});
purrdf_lex::json_record!(VizScenario as "explicit scenario" {
    "participants" => participants: required,
    "regions" => regions: required,
    "interactions" => interactions: required,
});
purrdf_lex::json_record!(VizPresentation as "visualization presentation" {
    "displays" => displays: required,
    "components" => components: required,
    "scenario" => scenario: optional,
});
purrdf_lex::json_record!(VizPresentedScene as "presented visualization scene" {
    "scene" => scene: required,
    "presentation" => presentation: required,
});
purrdf_lex::json_record!(VizPresentedExport as "presented visualization export" {
    "schema_version" => schema_version: required,
    "presentation" => presentation: required,
    "presentation_hash" => presentation_hash: required,
    "presented_scene_hash" => presented_scene_hash: required,
    "layout_options" => layout_options: required,
    "export" => export: required,
    "geometry" => geometry: required,
    "element_index" => element_index: required,
});
purrdf_lex::json_record!(VizPresentationFrame as "presentation frame" {
    "id" => id: required,
    "bindings" => bindings: required,
    "rect" => rect: required,
    "label" => label: required,
    "text" => text: required,
    "kind" => kind: optional,
    "parent" => parent: optional,
});
purrdf_lex::json_record!(VizBoundaryPort as "boundary port" {
    "id" => id: required,
    "boundary" => boundary: required,
    "edge" => edge: required,
    "point" => point: required,
    "bindings" => bindings: required,
});
purrdf_lex::json_record!(VizParticipantLayout as "participant geometry" {
    "id" => id: required,
    "term" => term: required,
    "rect" => rect: required,
    "label" => label: required,
    "text" => text: required,
    "start" => start: required,
    "end" => end: required,
});
purrdf_lex::json_record!(VizInteractionLayout as "interaction geometry" {
    "id" => id: required,
    "binding" => binding: required,
    "points" => points: required,
    "label" => label: required,
    "text" => text: required,
});
purrdf_lex::json_record!(VizPresentationLayout as "presentation geometry" {
    "components" => components: required,
    "ports" => ports: required,
    "participants" => participants: required,
    "regions" => regions: required,
    "interactions" => interactions: required,
    "detail_heading" => detail_heading: optional,
});
