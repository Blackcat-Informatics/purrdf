// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-owned presentation over the existing statement incidence model.

use super::{
    VizAccessibility, VizElementIndexEntry, VizError, VizExport, VizLayoutLabel, VizLayoutOptions,
    VizMode, VizPoint, VizProjection, VizRect, VizRelation, VizScene, VizSceneLabel, VizSceneNode,
    VizSemanticRef, VizSpec, VizStatement, VizTermId, VizValueRef, build_scene,
};
use std::collections::{BTreeMap, BTreeSet};

mod json;

/// Version of the enriched visualization export.
pub const VIZ_PRESENTED_SCHEMA_VERSION: &str = "purrdf-viz-presented-1";

/// A navigation reference admitted for an SVG link.
///
/// Only HTTP(S) URLs with an authority, or local relative/fragment references,
/// are admitted. Network-path references, controls and backslashes are refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizNavigation(String);

impl VizNavigation {
    /// Admit a caller-selected destination without interpreting an RDF IRI as a link.
    pub fn parse(value: &str) -> Result<Self, VizError> {
        if value.len() > 8192 {
            return Err(VizError::TooLarge {
                limit: "navigation bytes",
                actual: value.len(),
                allowed: 8192,
            });
        }
        let parsed = purrdf_iri::parse(value).map_err(|_| {
            presentation_error(
                VizPresentationIssue::Navigation,
                value,
                "use an HTTP(S) URL or local reference",
            )
        })?;
        let safe_scheme = parsed.scheme().is_none_or(|scheme| {
            (scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
                && parsed
                    .authority()
                    .is_some_and(|authority| !authority.is_empty())
        });
        if value.is_empty()
            || !safe_scheme
            || value.starts_with("//")
            || value
                .chars()
                .any(|c| c.is_control() || c == '\\' || c.is_whitespace())
        {
            return Err(presentation_error(
                VizPresentationIssue::Navigation,
                value,
                "use an HTTP(S) URL or local reference without controls, whitespace or backslashes",
            ));
        }
        Ok(Self(parsed.into_text()))
    }

    /// The admitted reference, for escaped carrier emission.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The reason a presentation instruction cannot be applied faithfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VizPresentationIssue {
    /// Missing, duplicate or ambiguous semantic identity.
    Binding,
    /// Invalid component membership or containment.
    Containment,
    /// Invalid participant, ordering or region structure.
    Scenario,
    /// An unsafe navigation destination.
    Navigation,
    /// An instruction unsupported by the requested RDF view.
    Unsupported,
    /// A stale or inconsistent enriched export.
    Export,
}

/// A typed, actionable presentation refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizPresentationError {
    /// Closed refusal class.
    pub issue: VizPresentationIssue,
    /// The instruction or semantic identity involved.
    pub instruction: String,
    /// How to correct the instruction.
    pub message: String,
}

/// Human-facing text and navigation attached to an exact semantic identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizDisplay {
    /// Identity whose native RDF scene representations receive this instruction.
    /// Explicit scenario participant and occurrence labels have their own fields.
    pub binding: VizSemanticRef,
    /// Display label; original RDF spelling remains in the model and accessibility.
    pub label: Option<VizSceneLabel>,
    /// Additional accessible description, appended to the semantic description.
    pub description: Option<String>,
    /// Caller-owned domain role, shown as text rather than inferred vocabulary.
    pub role: Option<String>,
    /// Explicit navigation, independent of the identity's RDF IRI.
    pub navigation: Option<VizNavigation>,
}

/// One component boundary in a caller-declared containment forest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizComponent {
    /// Stable caller-selected presentation identifier.
    pub id: String,
    /// Semantic node represented by the boundary's header.
    pub binding: VizSemanticRef,
    /// Parent component identifier, or the canvas.
    pub parent: Option<String>,
    /// Direct node members; child component membership comes from `parent`.
    pub members: Vec<VizSemanticRef>,
    /// Visible and accessible boundary label.
    pub label: VizSceneLabel,
}

/// A participant in explicit left-to-right scenario order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizParticipant {
    /// Exact RDF term; no participant is inferred from an edge order.
    pub term: VizTermId,
    /// Caller-selected participant name.
    pub label: VizSceneLabel,
    /// Caller-selected role, such as actor or service, rendered as text.
    pub role: Option<String>,
}

/// Structural region grammar; arrays never imply runtime order on their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VizRegionKind {
    /// Ordered child interactions and regions.
    Sequence,
    /// Two or more guarded branch/error arms.
    Alternative,
    /// An ordinary guarded alternative arm.
    Branch,
    /// An explicitly declared error arm.
    Error,
    /// Two or more concurrent lanes, with no inter-lane execution order.
    Parallel,
    /// An ordered lane within a parallel region.
    Lane,
}

/// A flat region record, avoiding recursive ownership and stack-sensitive decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizScenarioRegion {
    /// Stable presentation identifier.
    pub id: String,
    /// Containing region, or the implicit root sequence.
    pub parent: Option<String>,
    /// Explicit zero-based position among siblings.
    pub order: u32,
    /// Region's execution grammar.
    pub kind: VizRegionKind,
    /// Condition, lane name or region explanation.
    pub label: VizSceneLabel,
}

/// An explicitly ordered occurrence of a semantically bound interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizInteraction {
    /// Stable occurrence ID; repeated occurrences may share an RDF identity.
    pub id: String,
    /// Structural statement, assertion or annotation relation.
    pub binding: VizSemanticRef,
    /// Sending participant.
    pub from: VizTermId,
    /// Receiving participant, possibly the sender for a self-interaction.
    pub to: VizTermId,
    /// Containing sequence/arm/lane, or the implicit root sequence.
    pub region: Option<String>,
    /// Explicit zero-based position among siblings.
    pub order: u32,
    /// Caller display label; absent means the bound predicate's label.
    pub label: Option<VizSceneLabel>,
}

/// Explicit scenario structure. No ordering is extracted from RDF insertion order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VizScenario {
    /// Named participants, in left-to-right order.
    pub participants: Vec<VizParticipant>,
    /// Flat, bounded containment tree of structural regions.
    pub regions: Vec<VizScenarioRegion>,
    /// Interaction occurrences carrying explicit sibling positions.
    pub interactions: Vec<VizInteraction>,
}

/// Caller-selected presentation instructions; the empty lens preserves the RDF view.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VizPresentation {
    /// Semantic-keyed human-facing text and links.
    pub displays: Vec<VizDisplay>,
    /// Compound component boundaries.
    pub components: Vec<VizComponent>,
    /// Optional explicit scenario, rendered with a visible RDF detail view.
    pub scenario: Option<VizScenario>,
}

/// An enriched scene, without renderer-specific geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizPresentedScene {
    /// Complete native RDF scene, with display choices applied.
    pub scene: VizScene,
    /// Validated normalized instructions; ordering fields remain explicit.
    pub presentation: VizPresentation,
}

/// A positioned boundary or explicit scenario region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizPresentationFrame {
    /// Provider-owned stable scene identifier.
    pub id: String,
    /// Semantic identities carried by this frame.
    pub bindings: Vec<VizSemanticRef>,
    /// Outer frame rectangle.
    pub rect: VizRect,
    /// Full label and optional language/direction.
    pub label: VizSceneLabel,
    /// Measured label geometry.
    pub text: VizLayoutLabel,
    /// Explicit region grammar, absent for architectural boundaries.
    pub kind: Option<VizRegionKind>,
    /// Parent frame identifier, absent for roots.
    pub parent: Option<String>,
}

/// A boundary crossing with a real attachment on a component perimeter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizBoundaryPort {
    /// Stable provider-generated crossing identifier.
    pub id: String,
    /// Component frame identifier.
    pub boundary: String,
    /// Original scene edge using the crossing.
    pub edge: String,
    /// Point on the frame perimeter.
    pub point: VizPoint,
    /// Original semantic bindings, without a fabricated containment assertion.
    pub bindings: Vec<VizSemanticRef>,
}

/// A named participant and its lifeline geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizParticipantLayout {
    /// Stable scene identifier.
    pub id: String,
    /// Participant identity.
    pub term: VizTermId,
    /// Header rectangle.
    pub rect: VizRect,
    /// Header label, including any caller role.
    pub label: VizSceneLabel,
    /// Header text geometry.
    pub text: VizLayoutLabel,
    /// Lifeline start.
    pub start: VizPoint,
    /// Lifeline end.
    pub end: VizPoint,
}

/// A routed interaction occurrence; its region/order remain in the presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizInteractionLayout {
    /// Stable scene identifier.
    pub id: String,
    /// Actual semantic identity, never an inferred runtime assertion.
    pub binding: VizSemanticRef,
    /// Orthogonal interaction route.
    pub points: Vec<VizPoint>,
    /// Full visible label, including its explicit ordering path.
    pub label: VizSceneLabel,
    /// Measured label geometry.
    pub text: VizLayoutLabel,
}

/// Geometry supplementing the complete native RDF layout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VizPresentationLayout {
    /// Nested architectural boundaries, parent before child.
    pub components: Vec<VizPresentationFrame>,
    /// Explicit architectural boundary crossings.
    pub ports: Vec<VizBoundaryPort>,
    /// Scenario participants in declared order.
    pub participants: Vec<VizParticipantLayout>,
    /// Scenario regions, parent before child.
    pub regions: Vec<VizPresentationFrame>,
    /// Interaction routes in explicit depth-first presentation order.
    pub interactions: Vec<VizInteractionLayout>,
    /// Visible heading for the retained RDF detail panel when a scenario is present.
    pub detail_heading: Option<VizLayoutLabel>,
}

/// Complete provider-built enriched export. All RDF facts remain visibly rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizPresentedExport {
    /// Enriched export contract version.
    pub schema_version: String,
    /// Normalized caller instructions.
    pub presentation: VizPresentation,
    /// Hash of the exact normalized presentation JSON.
    pub presentation_hash: String,
    /// Hash of the enriched scene and instructions together.
    pub presented_scene_hash: String,
    /// Admitted layout request, retained so complete exports can be verified.
    pub layout_options: VizLayoutOptions,
    /// Complete RDF model, scene, geometry, semantic hashes and element index.
    pub export: VizExport,
    /// Component and scenario geometry, separate from the semantic scene.
    pub geometry: VizPresentationLayout,
    /// Complete visible-element index, extending the nested native RDF index.
    pub element_index: Vec<VizElementIndexEntry>,
}

pub(super) fn presentation_error(
    issue: VizPresentationIssue,
    instruction: &str,
    message: &str,
) -> VizError {
    VizError::Presentation(VizPresentationError {
        issue,
        instruction: instruction.to_owned(),
        message: message.to_owned(),
    })
}

/// Build a presentation without allowing it to replace or reinterpret RDF facts.
/// Table decorations are additive plain text; directed/full labels, navigation,
/// component boundaries and scenarios require a graph mode.
pub fn build_presented_scene(
    projection: &VizProjection,
    spec: &VizSpec,
    presentation: &VizPresentation,
) -> Result<VizPresentedScene, VizError> {
    validate_presentation(projection, presentation)?;
    let mut scene = build_scene(projection, spec)?;
    let element_count = scene
        .nodes
        .len()
        .saturating_add(scene.edges.len())
        .saturating_add(scene.table.as_ref().map_or(0, |t| {
            t.fields.len().saturating_add(
                t.rows
                    .iter()
                    .map(|r| r.cells.len())
                    .fold(0usize, usize::saturating_add),
            )
        }));
    if element_count > 2048 {
        return Err(VizError::TooLarge {
            limit: "presented scene elements",
            actual: element_count,
            allowed: 2048,
        });
    }
    if scene.mode == VizMode::Table
        && (!presentation.components.is_empty()
            || presentation.scenario.is_some()
            || presentation.displays.iter().any(|d| {
                d.navigation.is_some()
                    || d.label.as_ref().is_some_and(|label| {
                        label.language.is_some()
                            || label.direction.is_some()
                            || label.full_text != label.text
                    })
            }))
    {
        return Err(presentation_error(
            VizPresentationIssue::Unsupported,
            "table",
            "table decorations are plain text; use compact or incidence mode for directed/full labels, navigation, components or scenarios",
        ));
    }
    let mut normalized = presentation.clone();
    normalized
        .displays
        .sort_by(|a, b| a.binding.cmp(&b.binding));
    normalized.components.sort_by(|a, b| a.id.cmp(&b.id));
    for component in &mut normalized.components {
        component.members.sort();
    }
    if let Some(scenario) = &mut normalized.scenario {
        scenario.regions.sort_by(|a, b| a.id.cmp(&b.id));
        scenario.interactions.sort_by(|a, b| a.id.cmp(&b.id));
    }
    apply_displays(&mut scene, &normalized)?;
    for component in &normalized.components {
        scene_node(&scene, &component.binding)?;
        for member in &component.members {
            scene_node(&scene, member)?;
        }
    }
    Ok(VizPresentedScene {
        scene,
        presentation: normalized,
    })
}

pub(super) fn scene_node<'a>(
    scene: &'a VizScene,
    binding: &VizSemanticRef,
) -> Result<&'a VizSceneNode, VizError> {
    let mut matches = scene
        .nodes
        .iter()
        .filter(|node| node.bindings.contains(binding));
    let node = matches.next().ok_or_else(|| {
        presentation_error(
            VizPresentationIssue::Binding,
            "component member",
            "choose an identity represented by a node in this RDF mode",
        )
    })?;
    if matches.next().is_some() {
        return Err(presentation_error(
            VizPresentationIssue::Binding,
            &node.id,
            "choose an unambiguous node identity",
        ));
    }
    Ok(node)
}

fn semantic_refs(model: &VizProjection) -> Result<BTreeSet<VizSemanticRef>, VizError> {
    let refs: BTreeSet<_> = model
        .terms
        .iter()
        .map(|x| VizSemanticRef::Term(x.id.clone()))
        .chain(
            model
                .statements
                .iter()
                .map(|x| VizSemanticRef::Statement(x.id.clone())),
        )
        .chain(
            model
                .assertions
                .iter()
                .map(|x| VizSemanticRef::Assertion(x.id.clone())),
        )
        .chain(model.relations.iter().map(|x| {
            VizSemanticRef::Relation(match x {
                VizRelation::Reifies { id, .. } | VizRelation::Annotation { id, .. } => id.clone(),
            })
        }))
        .chain(
            model
                .graphs
                .iter()
                .map(|x| VizSemanticRef::Graph(x.id.clone())),
        )
        .chain(
            model
                .references
                .iter()
                .map(|x| VizSemanticRef::Reference(x.id.clone())),
        )
        .chain(
            model
                .diagnostics
                .iter()
                .map(|x| VizSemanticRef::Diagnostic(x.id.clone())),
        )
        .collect();
    let count = model
        .terms
        .len()
        .saturating_add(model.statements.len())
        .saturating_add(model.assertions.len())
        .saturating_add(model.relations.len())
        .saturating_add(model.graphs.len())
        .saturating_add(model.references.len())
        .saturating_add(model.diagnostics.len());
    if refs.len() != count {
        return Err(presentation_error(
            VizPresentationIssue::Binding,
            "projection",
            "use a projection with unique, unambiguous semantic identities",
        ));
    }
    Ok(refs)
}

fn text_budget(text: &str, used: &mut usize) -> Result<(), VizError> {
    *used = used.checked_add(text.len()).ok_or(VizError::TooLarge {
        limit: "presentation text",
        actual: usize::MAX,
        allowed: 262_144,
    })?;
    if *used > 262_144
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
    {
        return Err(presentation_error(
            VizPresentationIssue::Unsupported,
            "presentation text",
            "use at most 262144 UTF-8 bytes without carrier-invalid controls",
        ));
    }
    Ok(())
}

fn label_budget(label: &VizSceneLabel, used: &mut usize) -> Result<(), VizError> {
    if label.text.len() > 4096 {
        return Err(VizError::TooLarge {
            limit: "display label bytes",
            actual: label.text.len(),
            allowed: 4096,
        });
    }
    text_budget(&label.text, used)?;
    text_budget(&label.full_text, used)?;
    if let Some(language) = &label.language {
        text_budget(language, used)?;
    }
    Ok(())
}

fn validate_presentation(model: &VizProjection, p: &VizPresentation) -> Result<(), VizError> {
    let count = p
        .displays
        .len()
        .saturating_add(p.components.len())
        .saturating_add(
            p.components
                .iter()
                .map(|c| c.members.len())
                .fold(0usize, usize::saturating_add),
        )
        .saturating_add(p.scenario.as_ref().map_or(0, |s| {
            s.participants
                .len()
                .saturating_add(s.regions.len())
                .saturating_add(s.interactions.len())
        }));
    if count > 2048 {
        return Err(VizError::TooLarge {
            limit: "presentation instructions",
            actual: count,
            allowed: 2048,
        });
    }
    let known = semantic_refs(model)?;
    let mut seen = BTreeSet::new();
    let mut used = 0;
    for display in &p.displays {
        if !known.contains(&display.binding) || !seen.insert(&display.binding) {
            return Err(presentation_error(
                VizPresentationIssue::Binding,
                "display",
                "use each existing semantic identity once",
            ));
        }
        if let Some(label) = &display.label {
            label_budget(label, &mut used)?;
        }
        for text in display.description.iter().chain(display.role.iter()) {
            text_budget(text, &mut used)?;
        }
        if let Some(link) = &display.navigation {
            text_budget(link.as_str(), &mut used)?;
        }
    }
    validate_components(p, &known, &mut used)?;
    if let Some(scenario) = &p.scenario {
        validate_scenario(model, scenario, &mut used)?;
    }
    Ok(())
}

fn id_check(id: &str, used: &mut usize) -> Result<(), VizError> {
    text_budget(id, used)?;
    if id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(presentation_error(
            VizPresentationIssue::Binding,
            id,
            "use a nonempty ASCII alphanumeric, dash or underscore identifier",
        ));
    }
    Ok(())
}

fn validate_components(
    p: &VizPresentation,
    known: &BTreeSet<VizSemanticRef>,
    used: &mut usize,
) -> Result<(), VizError> {
    let mut ids = BTreeMap::new();
    let mut owners = BTreeSet::new();
    let mut members = BTreeSet::new();
    for c in &p.components {
        id_check(&c.id, used)?;
        label_budget(&c.label, used)?;
        if ids.insert(c.id.as_str(), c).is_some()
            || !known.contains(&c.binding)
            || !owners.insert(&c.binding)
        {
            return Err(presentation_error(
                VizPresentationIssue::Containment,
                &c.id,
                "use unique components bound to existing distinct nodes",
            ));
        }
    }
    for c in &p.components {
        for member in &c.members {
            if !known.contains(member) || owners.contains(member) || !members.insert(member) {
                return Err(presentation_error(
                    VizPresentationIssue::Containment,
                    &c.id,
                    "assign an existing non-component node to exactly one component",
                ));
            }
        }
        let mut cursor = Some(c.id.as_str());
        let mut path = BTreeSet::new();
        while let Some(id) = cursor {
            if !path.insert(id) || path.len() > 64 {
                return Err(presentation_error(
                    VizPresentationIssue::Containment,
                    &c.id,
                    "use an acyclic containment forest no deeper than 64",
                ));
            }
            cursor = ids
                .get(id)
                .ok_or_else(|| {
                    presentation_error(
                        VizPresentationIssue::Containment,
                        id,
                        "declare the parent component",
                    )
                })?
                .parent
                .as_deref();
        }
    }
    Ok(())
}

fn validate_scenario(
    model: &VizProjection,
    s: &VizScenario,
    used: &mut usize,
) -> Result<(), VizError> {
    let mut participants = BTreeSet::new();
    for participant in &s.participants {
        if !model.terms.iter().any(|t| t.id == participant.term)
            || !participants.insert(&participant.term)
        {
            return Err(presentation_error(
                VizPresentationIssue::Scenario,
                &participant.term.0,
                "declare each existing participant term exactly once",
            ));
        }
        label_budget(&participant.label, used)?;
        if let Some(role) = &participant.role {
            text_budget(role, used)?;
        }
    }
    if participants.is_empty() || s.interactions.is_empty() {
        return Err(presentation_error(
            VizPresentationIssue::Scenario,
            "scenario",
            "declare participants and at least one interaction",
        ));
    }
    let mut regions = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut children: BTreeMap<Option<&str>, Vec<(u32, Option<VizRegionKind>)>> = BTreeMap::new();
    for region in &s.regions {
        id_check(&region.id, used)?;
        label_budget(&region.label, used)?;
        if !ids.insert(region.id.as_str()) {
            return Err(presentation_error(
                VizPresentationIssue::Scenario,
                &region.id,
                "use distinct region and occurrence identifiers",
            ));
        }
        regions.insert(region.id.as_str(), region);
        children
            .entry(region.parent.as_deref())
            .or_default()
            .push((region.order, Some(region.kind)));
    }
    for region in &s.regions {
        let mut cursor = Some(region.id.as_str());
        let mut path = BTreeSet::new();
        while let Some(id) = cursor {
            if !path.insert(id) || path.len() > 64 {
                return Err(presentation_error(
                    VizPresentationIssue::Scenario,
                    id,
                    "use an acyclic region tree no deeper than 64",
                ));
            }
            cursor = regions
                .get(id)
                .ok_or_else(|| {
                    presentation_error(
                        VizPresentationIssue::Scenario,
                        id,
                        "declare the parent region",
                    )
                })?
                .parent
                .as_deref();
        }
        let parent_kind = region
            .parent
            .as_deref()
            .and_then(|id| regions.get(id))
            .map(|r| r.kind);
        let arm = matches!(region.kind, VizRegionKind::Branch | VizRegionKind::Error);
        let lane = region.kind == VizRegionKind::Lane;
        if arm != (parent_kind == Some(VizRegionKind::Alternative))
            || lane != (parent_kind == Some(VizRegionKind::Parallel))
            || ((arm || lane) && region.label.text.trim().is_empty())
        {
            return Err(presentation_error(
                VizPresentationIssue::Scenario,
                &region.id,
                "alternative children must be named branch/error arms; parallel children must be named lanes",
            ));
        }
    }
    for interaction in &s.interactions {
        id_check(&interaction.id, used)?;
        if !ids.insert(interaction.id.as_str())
            || !participants.contains(&interaction.from)
            || !participants.contains(&interaction.to)
        {
            return Err(presentation_error(
                VizPresentationIssue::Scenario,
                &interaction.id,
                "use distinct occurrence IDs and declared endpoints",
            ));
        }
        let (from, to, _) = interaction_values(model, &interaction.binding)?;
        if from != interaction.from || to != interaction.to {
            return Err(presentation_error(
                VizPresentationIssue::Binding,
                &interaction.id,
                "interaction endpoints must equal the bound RDF subject and object terms",
            ));
        }
        if let Some(label) = &interaction.label {
            label_budget(label, used)?;
        }
        if interaction
            .region
            .as_deref()
            .is_some_and(|id| !regions.contains_key(id))
        {
            return Err(presentation_error(
                VizPresentationIssue::Scenario,
                &interaction.id,
                "declare the containing region",
            ));
        }
        children
            .entry(interaction.region.as_deref())
            .or_default()
            .push((interaction.order, None));
    }
    for (parent, mut entries) in children {
        entries.sort_by_key(|entry| entry.0);
        if entries
            .iter()
            .enumerate()
            .any(|(i, (order, _))| usize::try_from(*order).ok() != Some(i))
        {
            return Err(presentation_error(
                VizPresentationIssue::Scenario,
                parent.unwrap_or("root"),
                "use contiguous distinct sibling positions starting at zero",
            ));
        }
        if let Some(region) = parent.and_then(|id| regions.get(id)) {
            let special = matches!(
                region.kind,
                VizRegionKind::Alternative | VizRegionKind::Parallel
            );
            if special && (entries.len() < 2 || entries.iter().any(|(_, kind)| kind.is_none())) {
                return Err(presentation_error(
                    VizPresentationIssue::Scenario,
                    &region.id,
                    "declare at least two arms or lanes, with interactions inside them",
                ));
            }
        }
    }
    if s.regions.iter().any(|r| {
        !s.interactions
            .iter()
            .any(|i| i.region.as_deref() == Some(&r.id))
            && !s
                .regions
                .iter()
                .any(|child| child.parent.as_deref() == Some(&r.id))
    }) {
        return Err(presentation_error(
            VizPresentationIssue::Scenario,
            "region",
            "remove empty regions or declare their contents",
        ));
    }
    Ok(())
}

pub(super) fn interaction_values(
    model: &VizProjection,
    binding: &VizSemanticRef,
) -> Result<(VizTermId, VizTermId, VizTermId), VizError> {
    let statement = match binding {
        VizSemanticRef::Statement(id) => model.statements.iter().find(|s| &s.id == id),
        VizSemanticRef::Assertion(id) => model
            .assertions
            .iter()
            .find(|a| &a.id == id)
            .and_then(|a| model.statements.iter().find(|s| s.id == a.statement)),
        VizSemanticRef::Relation(id) => {
            if let Some(VizRelation::Annotation {
                reifier,
                predicate,
                object: VizValueRef::Term { id: object },
                ..
            }) = model.relations.iter().find(
                |r| matches!(r, VizRelation::Annotation { id: candidate, .. } if candidate == id),
            ) {
                return Ok((reifier.clone(), object.clone(), predicate.clone()));
            }
            None
        }
        _ => None,
    };
    if let Some(VizStatement {
        subject: VizValueRef::Term { id: from },
        object: VizValueRef::Term { id: to },
        predicate,
        ..
    }) = statement
    {
        return Ok((from.clone(), to.clone(), predicate.clone()));
    }
    Err(presentation_error(
        VizPresentationIssue::Unsupported,
        "interaction",
        "bind a statement/assertion or annotation with ordinary term endpoints; inspect other RDF structure in the visible detail view",
    ))
}

fn apply_display(
    label: &mut VizSceneLabel,
    accessibility: &mut VizAccessibility,
    display: &VizDisplay,
) {
    if let Some(value) = &display.label
        && label != value
    {
        accessibility.description.push_str("\nOriginal RDF text: ");
        accessibility.description.push_str(&label.full_text);
        label.clone_from(value);
        accessibility.title.clone_from(&value.full_text);
    }
    if let Some(description) = &display.description {
        accessibility.description.push('\n');
        accessibility.description.push_str(description);
    }
}

fn apply_roles(
    label: &mut VizSceneLabel,
    accessibility: &mut VizAccessibility,
    bindings: &[VizSemanticRef],
    p: &VizPresentation,
) {
    let mut roles = p
        .displays
        .iter()
        .filter(|d| bindings.contains(&d.binding))
        .filter_map(|d| d.role.as_ref())
        .peekable();
    if roles.peek().is_none() {
        return;
    }
    for role in roles {
        label.text = format!("[{role}] {}", label.text);
        label.full_text = format!("[{role}] {}", label.full_text);
    }
    accessibility.title.clone_from(&label.full_text);
}

fn apply_displays(scene: &mut VizScene, p: &VizPresentation) -> Result<(), VizError> {
    let mut table_bytes = scene.table.as_ref().map_or(0, |table| {
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .map(|cell| cell.text.len())
            .fold(0usize, usize::saturating_add)
    });
    admit_table_text(table_bytes)?;
    for (id, bindings) in scene
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.bindings.as_slice()))
        .chain(
            scene
                .edges
                .iter()
                .map(|edge| (edge.id.as_str(), edge.bindings.as_slice())),
        )
        .chain(
            scene
                .edges
                .iter()
                .filter_map(|edge| edge.anchor.as_ref())
                .map(|anchor| (anchor.id.as_str(), anchor.bindings.as_slice())),
        )
    {
        let mut labels = p
            .displays
            .iter()
            .filter(|display| bindings.contains(&display.binding))
            .filter_map(|display| display.label.as_ref());
        if let Some(first) = labels.next()
            && labels.any(|label| label != first)
        {
            return Err(presentation_error(
                VizPresentationIssue::Binding,
                id,
                "identities sharing this visible element have conflicting labels; supply one label or identical label instructions",
            ));
        }
    }
    for display in &p.displays {
        let mut represented = false;
        for node in &mut scene.nodes {
            if node.bindings.contains(&display.binding) {
                apply_display(&mut node.label, &mut node.accessibility, display);
                represented = true;
            }
        }
        for edge in &mut scene.edges {
            if edge.bindings.contains(&display.binding) {
                apply_display(&mut edge.label, &mut edge.accessibility, display);
                represented = true;
            }
            if let Some(anchor) = &mut edge.anchor
                && anchor.bindings.contains(&display.binding)
            {
                apply_display(&mut anchor.label, &mut anchor.accessibility, display);
                represented = true;
            }
        }
        if let Some(table) = &mut scene.table {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    if cell.bindings.contains(&display.binding) {
                        let added = display
                            .label
                            .as_ref()
                            .map_or(0, |label| label.text.len().saturating_add(" — ".len()))
                            .saturating_add(
                                display
                                    .role
                                    .as_ref()
                                    .map_or(0, |role| role.len().saturating_add(2)),
                            )
                            .saturating_add(
                                display
                                    .description
                                    .as_ref()
                                    .map_or(0, |description| description.len().saturating_add(1)),
                            );
                        table_bytes = table_bytes.saturating_add(added);
                        admit_table_text(table_bytes)?;
                        represented = true;
                        if let Some(label) = &display.label {
                            cell.text = format!("{} — {}", label.text, cell.text);
                        }
                        if let Some(role) = &display.role {
                            cell.text = format!("{role}: {}", cell.text);
                        }
                        if let Some(description) = &display.description {
                            cell.text.push('\n');
                            cell.text.push_str(description);
                        }
                    }
                }
            }
        }
        if !represented {
            return Err(presentation_error(
                VizPresentationIssue::Unsupported,
                "display",
                "choose an identity visibly represented in this mode; predicate-only terms may require incidence mode",
            ));
        }
    }
    for node in &mut scene.nodes {
        apply_roles(&mut node.label, &mut node.accessibility, &node.bindings, p);
    }
    for edge in &mut scene.edges {
        apply_roles(&mut edge.label, &mut edge.accessibility, &edge.bindings, p);
        if let Some(anchor) = &mut edge.anchor {
            apply_roles(
                &mut anchor.label,
                &mut anchor.accessibility,
                &anchor.bindings,
                p,
            );
        }
    }
    Ok(())
}

fn admit_table_text(bytes: usize) -> Result<(), VizError> {
    if bytes > 262_144 {
        return Err(VizError::TooLarge {
            limit: "presented table text",
            actual: bytes,
            allowed: 262_144,
        });
    }
    Ok(())
}
