// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Completeness and geometric admission of the native presentation supplement.

use super::super::{LINE_HEIGHT, validate_label_bounds};
use crate::viz::presentation::scene_node;
use crate::viz::{
    VizError, VizLayout, VizLayoutLabel, VizPoint, VizPresentationLayout, VizPresentedScene,
    VizRect,
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
        return Err(VizError::Layout(format!(
            "presentation rectangle {id} is empty or outside the complete canvas"
        )));
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
            return Err(VizError::Layout(
                "presentation label lines exceed their assigned height".to_owned(),
            ));
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
        return Err(VizError::Layout(
            "presentation geometry does not cover every admitted instruction".to_owned(),
        ));
    }
    let frames: BTreeMap<_, _> = g
        .components
        .iter()
        .chain(&g.regions)
        .map(|frame| (frame.id.as_str(), frame))
        .collect();
    if frames.len() != g.components.len() + g.regions.len() {
        return Err(VizError::Layout(
            "presentation geometry repeats a frame identity".to_owned(),
        ));
    }
    for frame in frames.values() {
        rectangle(&frame.id, frame.rect, canvas)?;
        if let Some(parent) = &frame.parent {
            let parent = frames.get(parent.as_str()).ok_or_else(|| {
                VizError::Layout(format!(
                    "presentation frame {} has no parent geometry",
                    frame.id
                ))
            })?;
            if !contains(parent.rect, frame.rect) {
                return Err(VizError::Layout(format!(
                    "presentation frame {} leaves its parent boundary",
                    frame.id
                )));
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
            .ok_or_else(|| VizError::Layout("component frame identity is absent".to_owned()))?;
        for binding in std::iter::once(&component.binding).chain(&component.members) {
            let node = scene_node(&presented.scene, binding)?;
            let node = layout
                .nodes
                .iter()
                .find(|n| n.id == node.id)
                .ok_or_else(|| VizError::Layout("component node has no geometry".to_owned()))?;
            if !contains(frame.rect, node.rect) {
                return Err(VizError::Layout(format!(
                    "component {} does not contain its declared node {}",
                    component.id, node.id
                )));
            }
        }
    }
    for port in &g.ports {
        let frame = frames
            .get(port.boundary.as_str())
            .ok_or_else(|| VizError::Layout("boundary port has no frame".to_owned()))?;
        let rect = frame.rect;
        let on_boundary = ((port.point.x == rect.x || port.point.x == rect.right())
            && (rect.y..=rect.bottom()).contains(&port.point.y))
            || ((port.point.y == rect.y || port.point.y == rect.bottom())
                && (rect.x..=rect.right()).contains(&port.point.x));
        let edge = layout
            .edges
            .iter()
            .find(|edge| edge.id == port.edge)
            .ok_or_else(|| VizError::Layout("boundary port has no native RDF edge".to_owned()))?;
        if !on_boundary || !on_route(port.point, &edge.points) {
            return Err(VizError::Layout(format!(
                "boundary port {} is not on its perimeter and actual edge route",
                port.id
            )));
        }
    }
    for participant in &g.participants {
        rectangle(&participant.id, participant.rect, canvas)?;
        if participant.start.x != participant.end.x
            || participant.start.y >= participant.end.y
            || !point_inside(participant.start, canvas)
            || !point_inside(participant.end, canvas)
        {
            return Err(VizError::Layout(format!(
                "participant {} has an invalid lifeline",
                participant.id
            )));
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
            return Err(VizError::Layout(format!(
                "interaction {} has an invalid orthogonal route",
                interaction.id
            )));
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
