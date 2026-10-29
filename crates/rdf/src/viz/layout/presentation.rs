// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compound and explicitly ordered constraints on the native layout machinery.

use super::{
    ComponentLayout, WorkArc, WorkNode, WorkNodeKind, anchor_work_id, build_work_graph,
    chars_for_width, endpoint_work_id, finish_graph_layout, graph_minimum, layout_component,
    layout_graph_with_limit, layout_legend, layout_table_scene, measure_label, validate_layout,
    validate_options, wrap_text,
};
use crate::viz::presentation::{interaction_values, presentation_error, scene_node};
use crate::viz::{
    VizBoundaryPort, VizComponent, VizError, VizInteractionLayout, VizLayout, VizLayoutLabel,
    VizLayoutOptions, VizMode, VizParticipantLayout, VizPoint, VizPresentationFrame,
    VizPresentationIssue, VizPresentationLayout, VizPresentedScene, VizProjection, VizRect,
    VizRegionKind, VizScenario, VizScene, VizSceneLabel,
};
use std::collections::{BTreeMap, BTreeSet};

mod validate;

const PAD: i32 = 28;
const MAX_COORDINATE: i64 = 16_384;

fn coordinate(value: i64) -> Result<i32, VizError> {
    if !(0..=MAX_COORDINATE).contains(&value) {
        return Err(VizError::TooLarge {
            limit: "presentation coordinate",
            actual: usize::try_from(value).unwrap_or(usize::MAX),
            allowed: 16_384,
        });
    }
    i32::try_from(value)
        .map_err(|_| VizError::Layout("presentation coordinate cannot be represented".to_owned()))
}

fn add(a: i32, b: i32) -> Result<i32, VizError> {
    coordinate(i64::from(a) + i64::from(b))
}

pub(in crate::viz) fn layout_presented(
    model: &VizProjection,
    presented: &VizPresentedScene,
    options: &VizLayoutOptions,
) -> Result<(VizLayout, VizPresentationLayout), VizError> {
    validate_options(options)?;
    if [
        options.margin,
        options.rank_spacing,
        options.node_spacing,
        options.component_spacing,
        options.component_wrap_width,
        options.max_node_width,
    ]
    .iter()
    .any(|v| *v > 4096)
        || options.crossing_sweeps > 64
    {
        return Err(presentation_error(
            VizPresentationIssue::Unsupported,
            "layout",
            "presentation layout accepts distances up to 4096 and at most 64 crossing sweeps",
        ));
    }
    let scene = &presented.scene;
    let mut text_bytes = 0usize;
    for label in scene
        .nodes
        .iter()
        .map(|n| &n.label)
        .chain(scene.edges.iter().map(|e| &e.label))
    {
        if label.text.len() > 4096 {
            return Err(VizError::TooLarge {
                limit: "presented label bytes",
                actual: label.text.len(),
                allowed: 4096,
            });
        }
        text_bytes = text_bytes.saturating_add(label.text.len());
    }
    if text_bytes > 262_144 {
        return Err(VizError::TooLarge {
            limit: "presented scene text",
            actual: text_bytes,
            allowed: 262_144,
        });
    }
    let mut geometry = VizPresentationLayout::default();
    let mut layout = if presented.presentation.components.is_empty() {
        if scene.mode == VizMode::Table {
            layout_table_scene(scene, options)?
        } else {
            layout_graph_with_limit(scene, options, true)?
        }
    } else {
        compound(
            scene,
            &presented.presentation.components,
            options,
            &mut geometry,
        )?
    };
    if let Some(scenario) = &presented.presentation.scenario {
        let height = scenario_layout(model, scenario, options, &mut geometry)?;
        let dy = add(height, 90)?;
        translate_layout(&mut layout, 0, dy)?;
        translate_frames(&mut geometry.components, 0, dy)?;
        for port in &mut geometry.ports {
            port.point.y = add(port.point.y, dy)?;
        }
        geometry.detail_heading = Some(VizLayoutLabel {
            rect: VizRect {
                x: options.margin,
                y: add(height, 35)?,
                width: 600,
                height: 30,
            },
            lines: vec!["RDF detail — assertions, quotations and graph context".to_owned()],
        });
        layout.width = layout
            .width
            .max(
                geometry
                    .participants
                    .last()
                    .map_or(0, |p| p.rect.right() + 110),
            )
            .max(options.margin + 600);
        for frame in &geometry.regions {
            layout.width = layout.width.max(add(frame.rect.right(), options.margin)?);
        }
    }
    coordinate(i64::from(layout.width))?;
    coordinate(i64::from(layout.height))?;
    layout_legend(scene, &mut layout, options);
    coordinate(i64::from(layout.width))?;
    coordinate(i64::from(layout.height))?;
    validate_layout(scene, &layout)?;
    validate::geometry(presented, &layout, &geometry)?;
    Ok((layout, geometry))
}

struct ScopeLayout {
    layout: ComponentLayout,
    header: i32,
}

fn component_depth(id: &str, parents: &BTreeMap<String, String>) -> usize {
    let mut cursor = id;
    let mut depth = 0;
    while let Some(parent) = parents.get(cursor) {
        depth += 1;
        cursor = parent;
    }
    depth
}

fn representative(
    node: &str,
    scope: &str,
    owners: &BTreeMap<String, String>,
    parents: &BTreeMap<String, String>,
) -> Option<String> {
    let mut group = owners.get(node).map_or("", String::as_str);
    if group == scope {
        return Some(node.to_owned());
    }
    loop {
        let parent = parents.get(group).map_or("", String::as_str);
        if parent == scope {
            return Some(format!("present-component-{group}"));
        }
        if group.is_empty() {
            return None;
        }
        group = parent;
    }
}

fn compound(
    scene: &VizScene,
    components: &[VizComponent],
    options: &VizLayoutOptions,
    geometry: &mut VizPresentationLayout,
) -> Result<VizLayout, VizError> {
    let (work_nodes, work_arcs) = build_work_graph(scene, options)?;
    let mut owners = BTreeMap::new();
    let mut parents = BTreeMap::new();
    for c in components {
        parents.insert(c.id.clone(), c.parent.clone().unwrap_or_default());
        for binding in std::iter::once(&c.binding).chain(&c.members) {
            let node = scene_node(scene, binding)?;
            if owners.insert(node.id.clone(), c.id.clone()).is_some() {
                return Err(presentation_error(
                    VizPresentationIssue::Containment,
                    &c.id,
                    "assign each actual scene node to one component",
                ));
            }
        }
    }
    place_anchor_ownership(scene, &mut owners, &parents)?;
    let mut scopes: Vec<_> = components.iter().map(|c| c.id.clone()).collect();
    scopes.sort_by_key(|id| (std::cmp::Reverse(component_depth(id, &parents)), id.clone()));
    scopes.push(String::new());
    let mut local: BTreeMap<String, ScopeLayout> = BTreeMap::new();
    for scope in &scopes {
        let mut nodes = BTreeMap::new();
        for (id, node) in &work_nodes {
            if owners.get(id).map_or("", String::as_str) == scope {
                nodes.insert(id.clone(), node.clone());
            }
        }
        for (child, parent) in &parents {
            if parent == scope {
                let child_layout = &local[child];
                let declaration = components
                    .iter()
                    .find(|c| &c.id == child)
                    .ok_or_else(|| VizError::Layout("component declaration absent".to_owned()))?;
                let label_width =
                    presentation_measure(&declaration.label.text, options.max_node_width).0;
                nodes.insert(
                    format!("present-component-{child}"),
                    WorkNode {
                        kind: WorkNodeKind::Scene,
                        width: add(child_layout.layout.width.max(label_width), PAD * 2)?,
                        height: add(
                            add(child_layout.layout.height, child_layout.header)?,
                            PAD * 2,
                        )?,
                    },
                );
            }
        }
        let mut arcs = Vec::new();
        for arc in &work_arcs {
            if let (Some(source), Some(target)) = (
                representative(&arc.source, scope, &owners, &parents),
                representative(&arc.target, scope, &owners, &parents),
            ) && source != target
            {
                arcs.push(WorkArc {
                    source,
                    target,
                    ..arc.clone()
                });
            }
        }
        let ids = nodes.keys().cloned().collect();
        let layout = layout_component(&ids, &nodes, &arcs, options)?;
        coordinate(i64::from(layout.width))?;
        coordinate(i64::from(layout.height))?;
        let header = components.iter().find(|c| &c.id == scope).map_or(0, |c| {
            presentation_measure(&c.label.text, options.max_node_width).1 + PAD
        });
        local.insert(scope.clone(), ScopeLayout { layout, header });
    }
    let mut rects = BTreeMap::new();
    let mut ranks = BTreeMap::new();
    let mut stack = vec![(String::new(), options.margin, options.margin)];
    while let Some((scope, dx, dy)) = stack.pop() {
        let current = &local[&scope];
        for (id, rect) in &current.layout.positions {
            if let Some(child) = id.strip_prefix("present-component-") {
                let c = components.iter().find(|c| c.id == child).ok_or_else(|| {
                    VizError::Layout("component geometry lost its declaration".to_owned())
                })?;
                let outer = translate_rect(*rect, dx, dy)?;
                let label_rect = VizRect {
                    x: add(outer.x, PAD)?,
                    y: add(outer.y, 10)?,
                    width: outer.width - PAD * 2,
                    height: local[child].header - 10,
                };
                geometry.components.push(VizPresentationFrame {
                    id: id.clone(),
                    bindings: vec![c.binding.clone()],
                    rect: outer,
                    label: c.label.clone(),
                    text: text_geometry(&c.label, label_rect),
                    kind: None,
                    parent: c
                        .parent
                        .as_ref()
                        .map(|id| format!("present-component-{id}")),
                });
                stack.push((
                    child.to_owned(),
                    add(outer.x, PAD)?,
                    add(add(outer.y, PAD)?, local[child].header)?,
                ));
            } else {
                rects.insert(id.clone(), translate_rect(*rect, dx, dy)?);
                let rank = current.layout.ranks.get(id).copied().ok_or_else(|| {
                    VizError::Layout(format!("component layout omitted rank for {id}"))
                })?;
                ranks.insert(id.clone(), (0, rank));
            }
        }
    }
    geometry.components.sort_by_key(|frame| {
        (
            component_depth(
                frame
                    .id
                    .strip_prefix("present-component-")
                    .unwrap_or_default(),
                &parents,
            ),
            frame.id.clone(),
        )
    });
    let routes = boundary_routes(scene, &rects, &owners, &parents, geometry)?;
    let mut layout = finish_graph_layout(scene, options, &rects, &ranks, routes, true)?;
    let minimum = graph_minimum(&layout.nodes, &layout.edges, options.margin);
    let min_x = geometry
        .components
        .iter()
        .map(|frame| frame.rect.x)
        .min()
        .unwrap_or(minimum.x)
        .min(minimum.x);
    let min_y = geometry
        .components
        .iter()
        .map(|frame| frame.rect.y)
        .min()
        .unwrap_or(minimum.y)
        .min(minimum.y);
    let dx = (options.margin - min_x).max(0);
    let dy = (options.margin - min_y).max(0);
    translate_layout(&mut layout, dx, dy)?;
    translate_frames(&mut geometry.components, dx, dy)?;
    for port in &mut geometry.ports {
        port.point.x = add(port.point.x, dx)?;
        port.point.y = add(port.point.y, dy)?;
    }
    for frame in &geometry.components {
        layout.width = layout.width.max(add(frame.rect.right(), options.margin)?);
        layout.height = layout.height.max(add(frame.rect.bottom(), options.margin)?);
    }
    Ok(layout)
}

fn ancestry(
    node: &str,
    owners: &BTreeMap<String, String>,
    parents: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut path = Vec::new();
    let mut group = owners.get(node).map_or("", String::as_str);
    while !group.is_empty() {
        path.push(group.to_owned());
        group = parents.get(group).map_or("", String::as_str);
    }
    path
}

fn place_anchor_ownership(
    scene: &VizScene,
    owners: &mut BTreeMap<String, String>,
    parents: &BTreeMap<String, String>,
) -> Result<(), VizError> {
    for _ in 0..=scene.edges.len() {
        let mut changed = false;
        for edge in &scene.edges {
            let Some(anchor) = &edge.anchor else {
                continue;
            };
            let mut from = ancestry(&endpoint_work_id(&edge.source), owners, parents);
            let mut to = ancestry(&endpoint_work_id(&edge.target), owners, parents);
            let mut common = String::new();
            while from.last().is_some() && from.last() == to.last() {
                common = from
                    .pop()
                    .ok_or_else(|| VizError::Layout("anchor ancestry became empty".to_owned()))?;
                to.pop();
            }
            let id = anchor_work_id(&edge.id, &anchor.id);
            if owners.get(&id) != Some(&common) {
                owners.insert(id, common);
                changed = true;
            }
        }
        if !changed {
            return Ok(());
        }
    }
    Err(VizError::Layout(
        "statement-anchor containment did not reach a stable common ancestor".to_owned(),
    ))
}

fn boundary_routes(
    scene: &VizScene,
    rects: &BTreeMap<String, VizRect>,
    owners: &BTreeMap<String, String>,
    parents: &BTreeMap<String, String>,
    geometry: &mut VizPresentationLayout,
) -> Result<BTreeMap<String, Vec<VizPoint>>, VizError> {
    let frames: BTreeMap<_, _> = geometry
        .components
        .iter()
        .map(|frame| (frame.id.as_str(), frame.rect))
        .collect();
    let mut routes = BTreeMap::new();
    for (ordinal, edge) in scene.edges.iter().enumerate() {
        let source = endpoint_work_id(&edge.source);
        let target = endpoint_work_id(&edge.target);
        let start = rects
            .get(&source)
            .ok_or_else(|| VizError::Layout("missing compound source".to_owned()))?
            .center();
        let end = rects
            .get(&target)
            .ok_or_else(|| VizError::Layout("missing compound target".to_owned()))?
            .center();
        let mut from = ancestry(&source, owners, parents);
        let mut to = ancestry(&target, owners, parents);
        while from.last().is_some() && from.last() == to.last() {
            from.pop();
            to.pop();
        }
        let mut points = Vec::new();
        for (side, path) in [(false, from), (true, to.into_iter().rev().collect())] {
            if side && let Some(anchor) = &edge.anchor {
                points.push(rects[&anchor_work_id(&edge.id, &anchor.id)].center());
            }
            for component in path {
                let boundary = format!("present-component-{component}");
                let rect = frames[boundary.as_str()];
                let toward = if side { start } else { end };
                let point = VizPoint {
                    x: if toward.x >= rect.center().x {
                        rect.right()
                    } else {
                        rect.x
                    },
                    y: (if side { end.y } else { start.y }
                        + i32::try_from(ordinal % 7).unwrap_or_default() * 5)
                        .clamp(rect.y + PAD, rect.bottom() - PAD),
                };
                geometry.ports.push(VizBoundaryPort {
                    id: format!("{boundary}-{}-{}", edge.id, if side { "in" } else { "out" }),
                    boundary,
                    edge: edge.id.clone(),
                    point,
                    bindings: edge.bindings.clone(),
                });
                points.push(point);
            }
        }
        routes.insert(edge.id.clone(), points);
    }
    Ok(routes)
}

fn text_geometry(label: &VizSceneLabel, rect: VizRect) -> VizLayoutLabel {
    VizLayoutLabel {
        rect,
        lines: wrap_text(&label.text, chars_for_width(rect.width)),
    }
}

fn presentation_measure(value: &str, width: i32) -> (i32, i32) {
    measure_label(value, chars_for_width(width - 16), width)
}

fn translate_rect(rect: VizRect, dx: i32, dy: i32) -> Result<VizRect, VizError> {
    Ok(VizRect {
        x: add(rect.x, dx)?,
        y: add(rect.y, dy)?,
        ..rect
    })
}

fn translate_frames(frames: &mut [VizPresentationFrame], dx: i32, dy: i32) -> Result<(), VizError> {
    for frame in frames {
        frame.rect = translate_rect(frame.rect, dx, dy)?;
        frame.text.rect = translate_rect(frame.text.rect, dx, dy)?;
    }
    Ok(())
}

fn translate_layout(layout: &mut VizLayout, dx: i32, dy: i32) -> Result<(), VizError> {
    for node in &mut layout.nodes {
        node.rect = translate_rect(node.rect, dx, dy)?;
        node.label.rect = translate_rect(node.label.rect, dx, dy)?;
        for badge in &mut node.badges {
            badge.rect = translate_rect(badge.rect, dx, dy)?;
        }
        for port in &mut node.ports {
            port.point.x = add(port.point.x, dx)?;
            port.point.y = add(port.point.y, dy)?;
        }
    }
    for edge in &mut layout.edges {
        for point in &mut edge.points {
            point.x = add(point.x, dx)?;
            point.y = add(point.y, dy)?;
        }
        edge.label.rect = translate_rect(edge.label.rect, dx, dy)?;
        for badge in &mut edge.badges {
            badge.rect = translate_rect(badge.rect, dx, dy)?;
        }
        if let Some(anchor) = &mut edge.anchor {
            anchor.rect = translate_rect(anchor.rect, dx, dy)?;
            anchor.label.rect = translate_rect(anchor.label.rect, dx, dy)?;
            for badge in &mut anchor.badges {
                badge.rect = translate_rect(badge.rect, dx, dy)?;
            }
        }
    }
    layout.width = add(layout.width, dx)?;
    layout.height = add(layout.height, dy)?;
    Ok(())
}

fn region_children(s: &VizScenario, parent: Option<&str>) -> Vec<(u32, bool, usize)> {
    let mut out: Vec<_> = s
        .regions
        .iter()
        .enumerate()
        .filter(|(_, r)| r.parent.as_deref() == parent)
        .map(|(i, r)| (r.order, true, i))
        .chain(
            s.interactions
                .iter()
                .enumerate()
                .filter(|(_, i)| i.region.as_deref() == parent)
                .map(|(i, m)| (m.order, false, i)),
        )
        .collect();
    out.sort_by_key(|entry| entry.0);
    out
}

fn scenario_layout(
    model: &VizProjection,
    s: &VizScenario,
    options: &VizLayoutOptions,
    geometry: &mut VizPresentationLayout,
) -> Result<i32, VizError> {
    let mut x = add(options.margin, options.max_node_width / 2 + PAD)?;
    let mut header_height = 0;
    let mut centers = BTreeMap::new();
    for (ordinal, p) in s.participants.iter().enumerate() {
        let mut label = p.label.clone();
        if let Some(role) = &p.role {
            label.text = format!("{}\n[{role}]", label.text);
            label.full_text = format!("{}\n[{role}]", label.full_text);
        }
        let (width, height) = presentation_measure(&label.text, options.max_node_width);
        let rect = VizRect {
            x,
            y: options.margin,
            width: width + 32,
            height: height + 24,
        };
        header_height = header_height.max(rect.bottom());
        centers.insert(&p.term, rect.center().x);
        geometry.participants.push(VizParticipantLayout {
            id: format!("present-participant-{ordinal}"),
            term: p.term.clone(),
            rect,
            text: text_geometry(
                &label,
                VizRect {
                    x: x + 16,
                    y: rect.y + 12,
                    width,
                    height,
                },
            ),
            label,
            start: VizPoint {
                x: rect.center().x,
                y: rect.bottom(),
            },
            end: VizPoint::default(),
        });
        x = add(rect.right(), 110)?;
    }
    let mut y = add(header_height, 40)?;
    let left = options.margin;
    let mut maximum_depth = 0;
    for region in &s.regions {
        let mut parent = region.parent.as_deref();
        let mut depth = 1;
        while let Some(id) = parent {
            depth += 1;
            parent = s
                .regions
                .iter()
                .find(|r| r.id == id)
                .and_then(|r| r.parent.as_deref());
        }
        maximum_depth = maximum_depth.max(depth);
    }
    let width = (x - left).max(maximum_depth * 16 + options.max_node_width);
    enum Visit {
        Enter(u32, bool, usize, String),
        Exit(usize),
    }
    let mut stack: Vec<_> = region_children(s, None)
        .into_iter()
        .rev()
        .map(|(order, region, index)| Visit::Enter(order, region, index, String::new()))
        .collect();
    while let Some(visit) = stack.pop() {
        match visit {
            Visit::Exit(index) => {
                y = add(y, PAD)?;
                geometry.regions[index].rect.height = y - geometry.regions[index].rect.y;
            }
            Visit::Enter(order, region, index, prefix) => {
                let path = if prefix.is_empty() {
                    (u64::from(order) + 1).to_string()
                } else {
                    format!("{prefix}.{}", u64::from(order) + 1)
                };
                if region {
                    let r = &s.regions[index];
                    let depth = path.matches('.').count();
                    let indent = i32::try_from(depth)
                        .map_err(|_| VizError::Layout("region depth overflow".to_owned()))?
                        * 8;
                    let mut label = r.label.clone();
                    label.text = format!("{} — {}", region_name(r.kind), label.text);
                    label.full_text = format!("{} — {}", region_name(r.kind), label.full_text);
                    let (_, height) =
                        presentation_measure(&label.text, (width - indent * 2 - 32).max(80));
                    let rect = VizRect {
                        x: left + indent,
                        y,
                        width: width - indent * 2,
                        height: 0,
                    };
                    let text_rect = VizRect {
                        x: rect.x + 16,
                        y: y + 8,
                        width: rect.width - 32,
                        height,
                    };
                    let frame_index = geometry.regions.len();
                    geometry.regions.push(VizPresentationFrame {
                        id: format!("present-region-{}", r.id),
                        bindings: Vec::new(),
                        rect,
                        text: text_geometry(&label, text_rect),
                        label,
                        kind: Some(r.kind),
                        parent: r.parent.as_ref().map(|id| format!("present-region-{id}")),
                    });
                    y = add(y, height + PAD)?;
                    stack.push(Visit::Exit(frame_index));
                    for (order, region, index) in region_children(s, Some(&r.id)).into_iter().rev()
                    {
                        stack.push(Visit::Enter(order, region, index, path.clone()));
                    }
                } else {
                    let interaction = &s.interactions[index];
                    let (_, _, predicate) = interaction_values(model, &interaction.binding)?;
                    let predicate = model
                        .terms
                        .iter()
                        .find(|t| t.id == predicate)
                        .ok_or_else(|| VizError::Layout("scenario predicate absent".to_owned()))?;
                    let mut label = interaction
                        .label
                        .clone()
                        .unwrap_or_else(|| VizSceneLabel::plain(&predicate.label));
                    label.text = format!("{path}: {}", label.text);
                    label.full_text = format!("{path}: {}", label.full_text);
                    let from = centers[&interaction.from];
                    let to = centers[&interaction.to];
                    let (label_width, height) =
                        presentation_measure(&label.text, options.max_node_width);
                    y = add(y, height + 16)?;
                    let points = if from == to {
                        vec![
                            VizPoint { x: from, y },
                            VizPoint {
                                x: add(from, 64)?,
                                y,
                            },
                            VizPoint {
                                x: add(from, 64)?,
                                y: add(y, 32)?,
                            },
                            VizPoint {
                                x: from,
                                y: add(y, 32)?,
                            },
                        ]
                    } else {
                        vec![VizPoint { x: from, y }, VizPoint { x: to, y }]
                    };
                    let rect = VizRect {
                        x: i32::midpoint(from, to) - label_width / 2,
                        y: y - height - 8,
                        width: label_width,
                        height,
                    };
                    geometry.interactions.push(VizInteractionLayout {
                        id: format!("present-interaction-{}", interaction.id),
                        binding: interaction.binding.clone(),
                        points,
                        text: text_geometry(&label, rect),
                        label,
                    });
                    y = add(y, 60)?;
                }
            }
        }
    }
    for frame in &mut geometry.regions {
        let region_id = frame
            .id
            .strip_prefix("present-region-")
            .ok_or_else(|| VizError::Layout("region geometry lost its identity".to_owned()))?;
        let mut bindings = BTreeSet::new();
        for interaction in &s.interactions {
            let mut parent = interaction.region.as_deref();
            while let Some(id) = parent {
                if id == region_id {
                    bindings.insert(interaction.binding.clone());
                    break;
                }
                parent = s
                    .regions
                    .iter()
                    .find(|r| r.id == id)
                    .and_then(|r| r.parent.as_deref());
            }
        }
        frame.bindings = bindings.into_iter().collect();
    }
    for participant in &mut geometry.participants {
        participant.end = VizPoint {
            x: participant.start.x,
            y,
        };
    }
    Ok(y)
}

pub(in crate::viz) fn region_name(kind: VizRegionKind) -> &'static str {
    match kind {
        VizRegionKind::Sequence => "sequence",
        VizRegionKind::Alternative => "alternative",
        VizRegionKind::Branch => "condition",
        VizRegionKind::Error => "error",
        VizRegionKind::Parallel => "parallel (no inter-lane order)",
        VizRegionKind::Lane => "concurrent lane",
    }
}
