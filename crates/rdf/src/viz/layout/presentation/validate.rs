// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Completeness and geometric admission of the native presentation supplement.

use super::super::{LINE_HEIGHT, validate_label_bounds};
use crate::viz::presentation::scene_node;
use crate::viz::{
    VizError, VizLayout, VizLayoutError, VizLayoutLabel, VizPoint, VizPresentationLayout,
    VizPresentedScene, VizRect,
};
use std::collections::BTreeMap;

fn contains(parent: VizRect, child: VizRect) -> bool {
    child.x >= parent.x
        && child.y >= parent.y
        && i64::from(child.x) + i64::from(child.width)
            <= i64::from(parent.x) + i64::from(parent.width)
        && i64::from(child.y) + i64::from(child.height)
            <= i64::from(parent.y) + i64::from(parent.height)
}

fn rectangle(id: &str, rect: VizRect, canvas: VizRect) -> Result<(), VizError> {
    if rect.width <= 0 || rect.height <= 0 || !contains(canvas, rect) {
        return Err(VizError::Layout(VizLayoutError::PresentationRectangle {
            id: id.to_owned(),
            rect,
            canvas,
        }));
    }
    Ok(())
}

fn labels<'a>(
    labels: impl Iterator<Item = (&'a VizLayoutLabel, VizRect)>,
    canvas: VizRect,
) -> Result<(), VizError> {
    for (label, parent) in labels {
        rectangle("label", label.rect, canvas)?;
        validate_label_bounds(std::iter::once((label, parent)))?;
        if i64::try_from(label.lines.len())
            .ok()
            .and_then(|n| n.checked_mul(i64::from(LINE_HEIGHT)))
            .is_none_or(|height| height > i64::from(label.rect.height))
        {
            return Err(VizError::Layout(VizLayoutError::LabelHeight {
                rect: label.rect,
                lines: label.lines.len(),
                line_height: LINE_HEIGHT,
            }));
        }
    }
    Ok(())
}

fn point_inside(point: VizPoint, canvas: VizRect) -> bool {
    point.x >= 0 && point.y >= 0 && point.x <= canvas.width && point.y <= canvas.height
}

fn on_route(point: VizPoint, points: &[VizPoint]) -> bool {
    points.windows(2).any(|pair| {
        (pair[0].x == point.x
            && pair[1].x == point.x
            && (pair[0].y.min(pair[1].y)..=pair[0].y.max(pair[1].y)).contains(&point.y))
            || (pair[0].y == point.y
                && pair[1].y == point.y
                && (pair[0].x.min(pair[1].x)..=pair[0].x.max(pair[1].x)).contains(&point.x))
    })
}

pub(super) fn geometry(
    presented: &VizPresentedScene,
    layout: &VizLayout,
    g: &VizPresentationLayout,
) -> Result<(), VizError> {
    let canvas = VizRect {
        x: 0,
        y: 0,
        width: layout.width,
        height: layout.height,
    };
    let p = &presented.presentation;
    if g.components.len() != p.components.len()
        || g.participants.len() != p.scenario.as_ref().map_or(0, |s| s.participants.len())
        || g.regions.len() != p.scenario.as_ref().map_or(0, |s| s.regions.len())
        || g.interactions.len() != p.scenario.as_ref().map_or(0, |s| s.interactions.len())
        || g.detail_heading.is_some() != p.scenario.is_some()
    {
        return Err(VizError::Layout(VizLayoutError::InstructionCoverage {
            components: g.components.len(),
            expected_components: p.components.len(),
            participants: g.participants.len(),
            expected_participants: p.scenario.as_ref().map_or(0, |s| s.participants.len()),
            regions: g.regions.len(),
            expected_regions: p.scenario.as_ref().map_or(0, |s| s.regions.len()),
            interactions: g.interactions.len(),
            expected_interactions: p.scenario.as_ref().map_or(0, |s| s.interactions.len()),
            detail_heading: g.detail_heading.is_some(),
            expected_detail_heading: p.scenario.is_some(),
        }));
    }
    let frames: BTreeMap<_, _> = g
        .components
        .iter()
        .chain(&g.regions)
        .map(|frame| (frame.id.as_str(), frame))
        .collect();
    if frames.len() != g.components.len() + g.regions.len() {
        return Err(VizError::Layout(VizLayoutError::DuplicateFrames {
            unique: frames.len(),
            components: g.components.len(),
            regions: g.regions.len(),
        }));
    }
    for frame in frames.values() {
        rectangle(&frame.id, frame.rect, canvas)?;
        if let Some(parent) = &frame.parent {
            let parent = frames.get(parent.as_str()).ok_or_else(|| {
                VizError::Layout(VizLayoutError::MissingParentFrame {
                    frame: frame.id.clone(),
                    parent: parent.clone(),
                })
            })?;
            if !contains(parent.rect, frame.rect) {
                return Err(VizError::Layout(VizLayoutError::FrameBounds {
                    frame: frame.id.clone(),
                    parent: parent.id.clone(),
                    rect: frame.rect,
                    parent_rect: parent.rect,
                }));
            }
        }
    }
    labels(
        frames.values().map(|frame| (&frame.text, frame.rect)),
        canvas,
    )?;
    for component in &p.components {
        let frame = frames
            .get(format!("present-component-{}", component.id).as_str())
            .ok_or_else(|| {
                VizError::Layout(VizLayoutError::MissingComponentFrame {
                    component: component.id.clone(),
                })
            })?;
        for binding in std::iter::once(&component.binding).chain(&component.members) {
            let node = scene_node(&presented.scene, binding)?;
            let node = layout
                .nodes
                .iter()
                .find(|n| n.id == node.id)
                .ok_or_else(|| {
                    VizError::Layout(VizLayoutError::MissingComponentNode {
                        component: component.id.clone(),
                        node: node.id.clone(),
                    })
                })?;
            if !contains(frame.rect, node.rect) {
                return Err(VizError::Layout(VizLayoutError::ComponentBounds {
                    component: component.id.clone(),
                    node: node.id.clone(),
                    frame: frame.rect,
                    rect: node.rect,
                }));
            }
        }
    }
    for port in &g.ports {
        let frame = frames.get(port.boundary.as_str()).ok_or_else(|| {
            VizError::Layout(VizLayoutError::MissingPortFrame {
                port: port.id.clone(),
                boundary: port.boundary.clone(),
            })
        })?;
        let rect = frame.rect;
        let on_boundary = ((port.point.x == rect.x || port.point.x == rect.right())
            && (rect.y..=rect.bottom()).contains(&port.point.y))
            || ((port.point.y == rect.y || port.point.y == rect.bottom())
                && (rect.x..=rect.right()).contains(&port.point.x));
        let edge = layout
            .edges
            .iter()
            .find(|edge| edge.id == port.edge)
            .ok_or_else(|| {
                VizError::Layout(VizLayoutError::MissingPortEdge {
                    port: port.id.clone(),
                    edge: port.edge.clone(),
                })
            })?;
        if !on_boundary || !on_route(port.point, &edge.points) {
            return Err(VizError::Layout(VizLayoutError::PortRoute {
                port: port.id.clone(),
                boundary: port.boundary.clone(),
                edge: port.edge.clone(),
                point: port.point,
                frame: rect,
                points: edge.points.clone(),
            }));
        }
    }
    for participant in &g.participants {
        rectangle(&participant.id, participant.rect, canvas)?;
        if participant.start.x != participant.end.x
            || participant.start.y >= participant.end.y
            || !point_inside(participant.start, canvas)
            || !point_inside(participant.end, canvas)
        {
            return Err(VizError::Layout(VizLayoutError::Lifeline {
                participant: participant.id.clone(),
                start: participant.start,
                end: participant.end,
                canvas,
            }));
        }
    }
    labels(g.participants.iter().map(|p| (&p.text, p.rect)), canvas)?;
    for interaction in &g.interactions {
        if interaction.points.len() < 2
            || interaction
                .points
                .iter()
                .any(|point| !point_inside(*point, canvas))
            || interaction
                .points
                .windows(2)
                .any(|pair| pair[0].x != pair[1].x && pair[0].y != pair[1].y)
        {
            return Err(VizError::Layout(VizLayoutError::InteractionRoute {
                interaction: interaction.id.clone(),
                points: interaction.points.clone(),
                canvas,
            }));
        }
    }
    labels(
        g.interactions.iter().map(|i| (&i.text, i.text.rect)),
        canvas,
    )?;
    labels(
        g.detail_heading.iter().map(|label| (label, label.rect)),
        canvas,
    )?;
    for legend in &layout.legend {
        rectangle(&legend.id, legend.rect, canvas)?;
    }
    Ok(())
}
