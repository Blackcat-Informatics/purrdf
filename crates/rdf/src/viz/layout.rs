// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic layered layout for renderer-neutral visualization scenes.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::*;

/// Version of the deterministic layout contract.
pub const VIZ_LAYOUT_SCHEMA_VERSION: &str = "purrdf-viz-layout-1";

const CHAR_WIDTH: i32 = 8;
const LINE_HEIGHT: i32 = 18;
const NODE_PAD_X: i32 = 16;
const NODE_PAD_Y: i32 = 12;

/// Deterministic layout options independent of SVG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutOptions {
    /// Outer canvas margin.
    pub margin: i32,
    /// Horizontal separation between graph ranks.
    pub rank_spacing: i32,
    /// Vertical separation between nodes in one rank.
    pub node_spacing: i32,
    /// Separation between connected components.
    pub component_spacing: i32,
    /// Preferred component packing width.
    pub component_wrap_width: i32,
    /// Crossing-reduction sweep count.
    pub crossing_sweeps: u32,
    /// Maximum visible node label width.
    pub max_node_width: i32,
}

impl Default for VizLayoutOptions {
    fn default() -> Self {
        Self {
            margin: 36,
            rank_spacing: 150,
            node_spacing: 44,
            component_spacing: 96,
            component_wrap_width: 1440,
            crossing_sweeps: 8,
            max_node_width: 300,
        }
    }
}

/// Integer point in layout units.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VizPoint {
    /// Horizontal coordinate.
    pub x: i32,
    /// Vertical coordinate.
    pub y: i32,
}

/// Integer rectangle in layout units.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizRect {
    /// Left coordinate.
    pub x: i32,
    /// Top coordinate.
    pub y: i32,
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

impl VizRect {
    fn right(self) -> i32 {
        self.x + self.width
    }

    fn bottom(self) -> i32 {
        self.y + self.height
    }

    fn center(self) -> VizPoint {
        VizPoint {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }

    fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    fn translated(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }
}

/// Positioned and wrapped text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutLabel {
    /// Bounding rectangle.
    pub rect: VizRect,
    /// Wrapped lines in display order.
    pub lines: Vec<String>,
}

/// Positioned badge keyed by its index in the scene element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutBadge {
    /// Badge index in the scene record.
    pub index: usize,
    /// Bounding rectangle.
    pub rect: VizRect,
}

/// Positioned node port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutPort {
    /// Node-local port id.
    pub id: String,
    /// Absolute port position.
    pub point: VizPoint,
}

/// Positioned scene node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutNode {
    /// Scene node id.
    pub id: String,
    /// Node rectangle.
    pub rect: VizRect,
    /// Label geometry.
    pub label: VizLayoutLabel,
    /// Port positions.
    pub ports: Vec<VizLayoutPort>,
    /// Badge geometry.
    pub badges: Vec<VizLayoutBadge>,
    /// Connected component ordinal.
    pub component: usize,
    /// Layer/rank ordinal.
    pub rank: usize,
}

/// Positioned edge anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutAnchor {
    /// Scene anchor id.
    pub id: String,
    /// Anchor rectangle.
    pub rect: VizRect,
    /// Label geometry.
    pub label: VizLayoutLabel,
    /// Badge geometry.
    pub badges: Vec<VizLayoutBadge>,
}

/// Routed scene edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutEdge {
    /// Scene edge id.
    pub id: String,
    /// Orthogonal polyline points.
    pub points: Vec<VizPoint>,
    /// Edge label geometry.
    pub label: VizLayoutLabel,
    /// Badge geometry.
    pub badges: Vec<VizLayoutBadge>,
    /// Positioned edge anchor, when present.
    pub anchor: Option<VizLayoutAnchor>,
}

/// Positioned table cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutTableCell {
    /// Row index; zero is the header.
    pub row: usize,
    /// Column index.
    pub column: usize,
    /// Cell rectangle.
    pub rect: VizRect,
    /// Cell label geometry.
    pub label: VizLayoutLabel,
}

/// Positioned statement table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutTable {
    /// Table rectangle.
    pub rect: VizRect,
    /// Header and data cells.
    pub cells: Vec<VizLayoutTableCell>,
}

/// Complete deterministic geometry for a semantic scene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayout {
    /// Layout schema version.
    pub schema_version: String,
    /// Scene mode.
    pub mode: VizMode,
    /// Canvas width.
    pub width: i32,
    /// Canvas height.
    pub height: i32,
    /// Positioned nodes.
    pub nodes: Vec<VizLayoutNode>,
    /// Routed edges.
    pub edges: Vec<VizLayoutEdge>,
    /// Positioned table for table mode.
    pub table: Option<VizLayoutTable>,
}

/// Lay out a renderer-neutral semantic scene deterministically.
pub fn layout_scene(scene: &VizScene, options: &VizLayoutOptions) -> Result<VizLayout, VizError> {
    validate_options(options)?;
    let layout = if scene.mode == VizMode::Table {
        layout_table_scene(scene, options)?
    } else {
        layout_graph_scene(scene, options)?
    };
    validate_layout(scene, &layout)?;
    Ok(layout)
}

#[derive(Debug, Clone)]
enum WorkNodeKind {
    Scene,
    Anchor,
    Dummy,
}

#[derive(Debug, Clone)]
struct WorkNode {
    kind: WorkNodeKind,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone)]
struct WorkArc {
    id: String,
    scene_edge: String,
    source: String,
    target: String,
}

#[derive(Debug, Clone)]
struct ComponentLayout {
    id: String,
    width: i32,
    height: i32,
    positions: BTreeMap<String, VizRect>,
    ranks: BTreeMap<String, usize>,
    edge_waypoints: BTreeMap<String, Vec<VizPoint>>,
}

fn layout_graph_scene(scene: &VizScene, options: &VizLayoutOptions) -> Result<VizLayout, VizError> {
    let (work_nodes, work_arcs) = build_work_graph(scene, options)?;
    let components = connected_components(&work_nodes, &work_arcs);
    let mut component_layouts = components
        .iter()
        .map(|component| layout_component(component, &work_nodes, &work_arcs, options))
        .collect::<Result<Vec<_>, _>>()?;
    component_layouts.sort_by(|left, right| left.id.cmp(&right.id));
    let offsets = pack_components(&component_layouts, options);

    let mut all_rects = BTreeMap::new();
    let mut all_ranks = BTreeMap::new();
    let mut all_waypoints: BTreeMap<String, Vec<VizPoint>> = BTreeMap::new();
    for (component_index, component) in component_layouts.iter().enumerate() {
        let (dx, dy) = offsets[component_index];
        for (id, rect) in &component.positions {
            all_rects.insert(id.clone(), rect.translated(dx, dy));
            if let Some(rank) = component.ranks.get(id) {
                all_ranks.insert(id.clone(), (component_index, *rank));
            }
        }
        for (edge, points) in &component.edge_waypoints {
            all_waypoints
                .entry(edge.clone())
                .or_default()
                .extend(points.iter().map(|point| VizPoint {
                    x: point.x + dx,
                    y: point.y + dy,
                }));
        }
    }

    let scene_nodes = scene
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut nodes = Vec::new();
    for (id, node) in &scene_nodes {
        let rect = *all_rects
            .get(*id)
            .ok_or_else(|| VizError::Layout(format!("layout omitted scene node {id}")))?;
        let (component, rank) = all_ranks
            .get(*id)
            .copied()
            .ok_or_else(|| VizError::Layout(format!("layout omitted rank for scene node {id}")))?;
        nodes.push(place_scene_node(node, rect, component, rank));
    }
    nodes.sort_by(|left, right| left.id.cmp(&right.id));

    let node_lookup = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let anchor_lookup = build_anchor_lookup(scene, &all_rects)?;
    let mut occupied_labels = nodes.iter().map(|node| node.rect).collect::<Vec<_>>();
    let parallel_offsets = parallel_edge_offsets(scene);
    let mut edges = Vec::new();
    for edge in &scene.edges {
        let start = endpoint_point(&edge.source, &node_lookup, &anchor_lookup)?;
        let end = endpoint_point(&edge.target, &node_lookup, &anchor_lookup)?;
        let mut waypoints = all_waypoints.remove(&edge.id).unwrap_or_default();
        waypoints.sort_by_key(|point| (point.x, point.y));
        if start.x > end.x {
            waypoints.reverse();
        }
        let offset = parallel_offsets.get(&edge.id).copied().unwrap_or_default();
        let anchor = edge.anchor.as_ref().map(|scene_anchor| {
            let rect = *anchor_lookup
                .get(&(edge.id.as_str(), scene_anchor.id.as_str()))
                .expect("validated anchor lookup");
            place_anchor(scene_anchor, rect)
        });
        let mut required = waypoints;
        if let Some(value) = &anchor {
            required.push(value.rect.center());
            required.sort_by_key(|point| {
                if start.x <= end.x {
                    (point.x, point.y)
                } else {
                    (-point.x, point.y)
                }
            });
        }
        let points = route_edge(start, end, &required, offset);
        let label_size = measure_label(&edge.label.text, 22, 210);
        let label_rect = place_edge_label(&points, label_size, &occupied_labels);
        occupied_labels.push(label_rect);
        let label = VizLayoutLabel {
            rect: label_rect,
            lines: wrap_text(&edge.label.text, chars_for_width(label_rect.width)),
        };
        let badges = place_badges(&edge.badges, label_rect.x, label_rect.bottom() + 4, 240);
        occupied_labels.extend(badges.iter().map(|badge| badge.rect));
        edges.push(VizLayoutEdge {
            id: edge.id.clone(),
            points,
            label,
            badges,
            anchor,
        });
    }
    edges.sort_by(|left, right| left.id.cmp(&right.id));
    normalize_graph_geometry(&mut nodes, &mut edges, options.margin);

    let max_right = nodes
        .iter()
        .map(|node| node.rect.right())
        .chain(edges.iter().flat_map(|edge| {
            std::iter::once(edge.label.rect.right())
                .chain(edge.badges.iter().map(|badge| badge.rect.right()))
        }))
        .max()
        .unwrap_or(options.margin);
    let max_bottom = nodes
        .iter()
        .map(|node| node.rect.bottom())
        .chain(edges.iter().flat_map(|edge| {
            std::iter::once(edge.label.rect.bottom())
                .chain(edge.badges.iter().map(|badge| badge.rect.bottom()))
        }))
        .max()
        .unwrap_or(options.margin);
    Ok(VizLayout {
        schema_version: VIZ_LAYOUT_SCHEMA_VERSION.to_owned(),
        mode: scene.mode,
        width: max_right + options.margin,
        height: max_bottom + options.margin,
        nodes,
        edges,
        table: None,
    })
}

fn normalize_graph_geometry(nodes: &mut [VizLayoutNode], edges: &mut [VizLayoutEdge], margin: i32) {
    let min_x = nodes
        .iter()
        .map(|node| node.rect.x)
        .chain(edges.iter().flat_map(|edge| {
            edge.points
                .iter()
                .map(|point| point.x)
                .chain(std::iter::once(edge.label.rect.x))
                .chain(edge.badges.iter().map(|badge| badge.rect.x))
                .chain(edge.anchor.iter().map(|anchor| anchor.rect.x))
        }))
        .min()
        .unwrap_or(margin);
    let min_y = nodes
        .iter()
        .map(|node| node.rect.y)
        .chain(edges.iter().flat_map(|edge| {
            edge.points
                .iter()
                .map(|point| point.y)
                .chain(std::iter::once(edge.label.rect.y))
                .chain(edge.badges.iter().map(|badge| badge.rect.y))
                .chain(edge.anchor.iter().map(|anchor| anchor.rect.y))
        }))
        .min()
        .unwrap_or(margin);
    let dx = margin - min_x;
    let dy = margin - min_y;
    if dx == 0 && dy == 0 {
        return;
    }
    for node in nodes {
        node.rect = node.rect.translated(dx, dy);
        node.label.rect = node.label.rect.translated(dx, dy);
        for port in &mut node.ports {
            port.point.x += dx;
            port.point.y += dy;
        }
        for badge in &mut node.badges {
            badge.rect = badge.rect.translated(dx, dy);
        }
    }
    for edge in edges {
        for point in &mut edge.points {
            point.x += dx;
            point.y += dy;
        }
        edge.label.rect = edge.label.rect.translated(dx, dy);
        for badge in &mut edge.badges {
            badge.rect = badge.rect.translated(dx, dy);
        }
        if let Some(anchor) = &mut edge.anchor {
            anchor.rect = anchor.rect.translated(dx, dy);
            anchor.label.rect = anchor.label.rect.translated(dx, dy);
            for badge in &mut anchor.badges {
                badge.rect = badge.rect.translated(dx, dy);
            }
        }
    }
}

fn build_work_graph(
    scene: &VizScene,
    options: &VizLayoutOptions,
) -> Result<(BTreeMap<String, WorkNode>, Vec<WorkArc>), VizError> {
    let mut nodes = BTreeMap::new();
    for node in &scene.nodes {
        let (width, height) = measure_node(node, options);
        nodes.insert(
            node.id.clone(),
            WorkNode {
                kind: WorkNodeKind::Scene,
                width,
                height,
            },
        );
    }
    for edge in &scene.edges {
        if let Some(anchor) = &edge.anchor {
            let id = anchor_work_id(&edge.id, &anchor.id);
            let (width, height) = measure_anchor(anchor);
            nodes.insert(
                id.clone(),
                WorkNode {
                    kind: WorkNodeKind::Anchor,
                    width,
                    height,
                },
            );
        }
    }
    let mut arcs = Vec::new();
    for edge in &scene.edges {
        let source = endpoint_work_id(&edge.source);
        let target = endpoint_work_id(&edge.target);
        if !nodes.contains_key(&source) || !nodes.contains_key(&target) {
            return Err(VizError::Layout(format!(
                "scene edge {} has an endpoint absent from the work graph",
                edge.id
            )));
        }
        if let Some(anchor) = &edge.anchor {
            let anchor_id = anchor_work_id(&edge.id, &anchor.id);
            arcs.push(WorkArc {
                id: format!("{}:0", edge.id),
                scene_edge: edge.id.clone(),
                source,
                target: anchor_id.clone(),
            });
            arcs.push(WorkArc {
                id: format!("{}:1", edge.id),
                scene_edge: edge.id.clone(),
                source: anchor_id,
                target,
            });
        } else {
            arcs.push(WorkArc {
                id: format!("{}:0", edge.id),
                scene_edge: edge.id.clone(),
                source,
                target,
            });
        }
    }
    arcs.sort_by(|left, right| left.id.cmp(&right.id));
    Ok((nodes, arcs))
}

fn endpoint_work_id(endpoint: &VizEndpoint) -> String {
    match endpoint {
        VizEndpoint::NodePort { node, .. } => node.clone(),
        VizEndpoint::EdgeAnchor { edge, anchor } => anchor_work_id(edge, anchor),
    }
}

fn anchor_work_id(edge: &str, anchor: &str) -> String {
    format!("work-anchor:{edge}:{anchor}")
}

fn connected_components(
    nodes: &BTreeMap<String, WorkNode>,
    arcs: &[WorkArc],
) -> Vec<BTreeSet<String>> {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = nodes
        .keys()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    for arc in arcs {
        adjacency
            .entry(arc.source.clone())
            .or_default()
            .insert(arc.target.clone());
        adjacency
            .entry(arc.target.clone())
            .or_default()
            .insert(arc.source.clone());
    }
    let mut unseen = nodes.keys().cloned().collect::<BTreeSet<_>>();
    let mut components = Vec::new();
    while let Some(seed) = unseen.first().cloned() {
        let mut component = BTreeSet::new();
        let mut queue = VecDeque::from([seed.clone()]);
        unseen.remove(&seed);
        while let Some(node) = queue.pop_front() {
            component.insert(node.clone());
            if let Some(neighbors) = adjacency.get(&node) {
                for neighbor in neighbors {
                    if unseen.remove(neighbor) {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
        components.push(component);
    }
    components
}

fn layout_component(
    component: &BTreeSet<String>,
    all_nodes: &BTreeMap<String, WorkNode>,
    all_arcs: &[WorkArc],
    options: &VizLayoutOptions,
) -> Result<ComponentLayout, VizError> {
    let mut nodes = component
        .iter()
        .filter_map(|id| all_nodes.get(id).map(|node| (id.clone(), node.clone())))
        .collect::<BTreeMap<_, _>>();
    let arcs = all_arcs
        .iter()
        .filter(|arc| component.contains(&arc.source) && component.contains(&arc.target))
        .cloned()
        .collect::<Vec<_>>();
    let oriented = orient_acyclic(component, &arcs);
    let ranks = assign_ranks(component, &oriented)?;
    let (mut layers, normalized, edge_chains) = normalize_long_edges(&mut nodes, &oriented, &ranks);
    reduce_crossings(&mut layers, &normalized, options.crossing_sweeps);
    let (positions, width, height) = assign_coordinates(&layers, &nodes, options);
    let mut edge_waypoints: BTreeMap<String, Vec<VizPoint>> = BTreeMap::new();
    for (arc_id, chain) in edge_chains {
        let Some(arc) = arcs.iter().find(|candidate| candidate.id == arc_id) else {
            continue;
        };
        edge_waypoints
            .entry(arc.scene_edge.clone())
            .or_default()
            .extend(chain.iter().filter_map(|id| {
                nodes.get(id).and_then(|node| {
                    matches!(node.kind, WorkNodeKind::Dummy)
                        .then(|| positions.get(id).copied().map(VizRect::center))
                        .flatten()
                })
            }));
    }
    let mut final_ranks = BTreeMap::new();
    for (rank, layer) in layers.iter().enumerate() {
        for id in layer {
            final_ranks.insert(id.clone(), rank);
        }
    }
    Ok(ComponentLayout {
        id: component.first().cloned().unwrap_or_default(),
        width,
        height,
        positions,
        ranks: final_ranks,
        edge_waypoints,
    })
}

fn orient_acyclic(component: &BTreeSet<String>, arcs: &[WorkArc]) -> Vec<(String, String, String)> {
    let mut outgoing: BTreeMap<String, Vec<&WorkArc>> = BTreeMap::new();
    for arc in arcs.iter().filter(|arc| arc.source != arc.target) {
        outgoing.entry(arc.source.clone()).or_default().push(arc);
    }
    for values in outgoing.values_mut() {
        values.sort_by(|left, right| {
            left.target
                .cmp(&right.target)
                .then_with(|| left.id.cmp(&right.id))
        });
    }
    let mut state: BTreeMap<String, u8> = component.iter().map(|id| (id.clone(), 0)).collect();
    let mut reversed = BTreeSet::new();
    for root in component {
        if state.get(root).copied().unwrap_or_default() == 0 {
            dfs_cycle_break(root, &outgoing, &mut state, &mut reversed);
        }
    }
    arcs.iter()
        .filter(|arc| arc.source != arc.target)
        .map(|arc| {
            if reversed.contains(&arc.id) {
                (arc.id.clone(), arc.target.clone(), arc.source.clone())
            } else {
                (arc.id.clone(), arc.source.clone(), arc.target.clone())
            }
        })
        .collect()
}

fn dfs_cycle_break(
    node: &str,
    outgoing: &BTreeMap<String, Vec<&WorkArc>>,
    state: &mut BTreeMap<String, u8>,
    reversed: &mut BTreeSet<String>,
) {
    state.insert(node.to_owned(), 1);
    if let Some(arcs) = outgoing.get(node) {
        for arc in arcs {
            match state.get(&arc.target).copied().unwrap_or_default() {
                0 => dfs_cycle_break(&arc.target, outgoing, state, reversed),
                1 => {
                    reversed.insert(arc.id.clone());
                }
                _ => {}
            }
        }
    }
    state.insert(node.to_owned(), 2);
}

fn assign_ranks(
    component: &BTreeSet<String>,
    oriented: &[(String, String, String)],
) -> Result<BTreeMap<String, usize>, VizError> {
    let mut indegree = component
        .iter()
        .map(|id| (id.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_, source, target) in oriented {
        *indegree.entry(target.clone()).or_default() += 1;
        outgoing
            .entry(source.clone())
            .or_default()
            .push(target.clone());
    }
    for targets in outgoing.values_mut() {
        targets.sort();
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect::<BTreeSet<_>>();
    let mut ranks = component
        .iter()
        .map(|id| (id.clone(), 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut visited = 0usize;
    while let Some(node) = ready.pop_first() {
        visited += 1;
        let source_rank = ranks.get(&node).copied().unwrap_or_default();
        if let Some(targets) = outgoing.get(&node) {
            for target in targets {
                ranks
                    .entry(target.clone())
                    .and_modify(|rank| *rank = (*rank).max(source_rank + 1));
                if let Some(degree) = indegree.get_mut(target) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(target.clone());
                    }
                }
            }
        }
    }
    if visited != component.len() {
        return Err(VizError::Layout(
            "deterministic cycle breaking did not produce a DAG".to_owned(),
        ));
    }
    Ok(ranks)
}

type NormalizedEdge = (String, String);
type Layers = Vec<Vec<String>>;
type EdgeChains = BTreeMap<String, Vec<String>>;

fn normalize_long_edges(
    nodes: &mut BTreeMap<String, WorkNode>,
    oriented: &[(String, String, String)],
    ranks: &BTreeMap<String, usize>,
) -> (Layers, Vec<NormalizedEdge>, EdgeChains) {
    let max_rank = ranks.values().copied().max().unwrap_or_default();
    let mut layers = vec![Vec::new(); max_rank + 1];
    for (id, rank) in ranks {
        layers[*rank].push(id.clone());
    }
    for layer in &mut layers {
        layer.sort();
    }
    let mut normalized = Vec::new();
    let mut chains = BTreeMap::new();
    for (arc_id, source, target) in oriented {
        let source_rank = ranks.get(source).copied().unwrap_or_default();
        let target_rank = ranks.get(target).copied().unwrap_or(source_rank + 1);
        let mut chain = vec![source.clone()];
        for (rank, layer) in layers
            .iter_mut()
            .enumerate()
            .take(target_rank)
            .skip(source_rank + 1)
        {
            let id = format!("dummy:{arc_id}:{rank}");
            nodes.insert(
                id.clone(),
                WorkNode {
                    kind: WorkNodeKind::Dummy,
                    width: 8,
                    height: 8,
                },
            );
            layer.push(id.clone());
            chain.push(id);
        }
        chain.push(target.clone());
        for pair in chain.windows(2) {
            normalized.push((pair[0].clone(), pair[1].clone()));
        }
        chains.insert(arc_id.clone(), chain);
    }
    for layer in &mut layers {
        layer.sort();
    }
    (layers, normalized, chains)
}

fn reduce_crossings(layers: &mut [Vec<String>], edges: &[NormalizedEdge], sweeps: u32) {
    if layers.len() < 2 {
        return;
    }
    let mut neighbors_up: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut neighbors_down: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (source, target) in edges {
        neighbors_down
            .entry(source.clone())
            .or_default()
            .push(target.clone());
        neighbors_up
            .entry(target.clone())
            .or_default()
            .push(source.clone());
    }
    for _ in 0..sweeps {
        for rank in 1..layers.len() {
            let previous = position_map(&layers[rank - 1]);
            sort_layer(&mut layers[rank], &neighbors_up, &previous);
        }
        for rank in (0..layers.len() - 1).rev() {
            let next = position_map(&layers[rank + 1]);
            sort_layer(&mut layers[rank], &neighbors_down, &next);
        }
    }
}

fn position_map(layer: &[String]) -> BTreeMap<String, i64> {
    layer
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), i64::try_from(index).unwrap_or(i64::MAX / 4)))
        .collect()
}

fn sort_layer(
    layer: &mut [String],
    neighbors: &BTreeMap<String, Vec<String>>,
    adjacent_positions: &BTreeMap<String, i64>,
) {
    let current = position_map(layer);
    layer.sort_by(|left, right| {
        let (left_sum, left_count) = barycenter(left, neighbors, adjacent_positions, &current);
        let (right_sum, right_count) = barycenter(right, neighbors, adjacent_positions, &current);
        (i128::from(left_sum) * i128::from(right_count))
            .cmp(&(i128::from(right_sum) * i128::from(left_count)))
            .then_with(|| left.cmp(right))
    });
}

fn barycenter(
    node: &str,
    neighbors: &BTreeMap<String, Vec<String>>,
    adjacent_positions: &BTreeMap<String, i64>,
    current: &BTreeMap<String, i64>,
) -> (i64, i64) {
    let positions = neighbors
        .get(node)
        .into_iter()
        .flatten()
        .filter_map(|neighbor| adjacent_positions.get(neighbor).copied())
        .collect::<Vec<_>>();
    if positions.is_empty() {
        (current.get(node).copied().unwrap_or_default(), 1)
    } else {
        (
            positions.iter().sum(),
            i64::try_from(positions.len()).unwrap_or(1),
        )
    }
}

fn assign_coordinates(
    layers: &[Vec<String>],
    nodes: &BTreeMap<String, WorkNode>,
    options: &VizLayoutOptions,
) -> (BTreeMap<String, VizRect>, i32, i32) {
    let rank_widths = layers
        .iter()
        .map(|layer| {
            layer
                .iter()
                .filter_map(|id| nodes.get(id).map(|node| node.width))
                .max()
                .unwrap_or(8)
        })
        .collect::<Vec<_>>();
    let layer_heights = layers
        .iter()
        .map(|layer| {
            layer
                .iter()
                .filter_map(|id| nodes.get(id))
                .map(|node| node.height)
                .sum::<i32>()
                + options.node_spacing * i32_from_usize(layer.len().saturating_sub(1))
        })
        .collect::<Vec<_>>();
    let max_height = layer_heights.iter().copied().max().unwrap_or(1);
    let mut rank_x = Vec::with_capacity(layers.len());
    let mut x = 0;
    for width in &rank_widths {
        rank_x.push(x);
        x += *width + options.rank_spacing;
    }
    let total_width = rank_x
        .last()
        .zip(rank_widths.last())
        .map_or(1, |(left, width)| left + width);
    let mut positions = BTreeMap::new();
    for (rank, layer) in layers.iter().enumerate() {
        let mut y = (max_height - layer_heights[rank]) / 2;
        for id in layer {
            let Some(node) = nodes.get(id) else {
                continue;
            };
            let rect = VizRect {
                x: rank_x[rank] + (rank_widths[rank] - node.width) / 2,
                y,
                width: node.width,
                height: node.height,
            };
            positions.insert(id.clone(), rect);
            y += node.height + options.node_spacing;
        }
    }
    (positions, total_width, max_height.max(1))
}

fn pack_components(components: &[ComponentLayout], options: &VizLayoutOptions) -> Vec<(i32, i32)> {
    let mut offsets = Vec::with_capacity(components.len());
    let mut x = options.margin;
    let mut y = options.margin;
    let mut row_height = 0;
    for component in components {
        if x > options.margin
            && x + component.width > options.component_wrap_width.max(options.margin * 2)
        {
            x = options.margin;
            y += row_height + options.component_spacing;
            row_height = 0;
        }
        offsets.push((x, y));
        x += component.width + options.component_spacing;
        row_height = row_height.max(component.height);
    }
    offsets
}

fn place_scene_node(
    scene: &VizSceneNode,
    rect: VizRect,
    component: usize,
    rank: usize,
) -> VizLayoutNode {
    let label_width = (rect.width - NODE_PAD_X * 2).max(CHAR_WIDTH);
    let lines = wrap_text(&scene.label.text, chars_for_width(label_width));
    let label_height = LINE_HEIGHT * i32_from_usize(lines.len());
    let label = VizLayoutLabel {
        rect: VizRect {
            x: rect.x + NODE_PAD_X,
            y: rect.y + NODE_PAD_Y,
            width: label_width,
            height: label_height,
        },
        lines,
    };
    let badges = place_badges(
        &scene.badges,
        rect.x + NODE_PAD_X,
        label.rect.bottom() + 8,
        rect.width - NODE_PAD_X * 2,
    );
    let ports = scene
        .ports
        .iter()
        .map(|port| VizLayoutPort {
            id: port.id.clone(),
            point: port_point(port.kind, rect),
        })
        .collect();
    VizLayoutNode {
        id: scene.id.clone(),
        rect,
        label,
        ports,
        badges,
        component,
        rank,
    }
}

fn measure_node(node: &VizSceneNode, options: &VizLayoutOptions) -> (i32, i32) {
    let max_width = match node.kind {
        VizSceneNodeKind::Statement => options.max_node_width.max(360),
        _ => options.max_node_width,
    };
    let min_width = match node.kind {
        VizSceneNodeKind::Statement => 220,
        VizSceneNodeKind::Literal => 130,
        VizSceneNodeKind::Iri | VizSceneNodeKind::Blank => 116,
    };
    let natural_width =
        i32_from_usize(node.label.text.chars().count()) * CHAR_WIDTH + NODE_PAD_X * 2;
    let width = natural_width.clamp(min_width, max_width);
    let lines = wrap_text(&node.label.text, chars_for_width(width - NODE_PAD_X * 2));
    let badge_rows = badge_row_count(&node.badges, width - NODE_PAD_X * 2);
    let height = NODE_PAD_Y * 2
        + LINE_HEIGHT * i32_from_usize(lines.len())
        + if badge_rows == 0 {
            0
        } else {
            8 + badge_rows * 22
        };
    (width, height.max(48))
}

fn place_anchor(anchor: &VizEdgeAnchor, rect: VizRect) -> VizLayoutAnchor {
    let label_rect = VizRect {
        x: rect.x + 8,
        y: rect.y + 6,
        width: (rect.width - 16).max(CHAR_WIDTH),
        height: LINE_HEIGHT,
    };
    VizLayoutAnchor {
        id: anchor.id.clone(),
        rect,
        label: VizLayoutLabel {
            rect: label_rect,
            lines: wrap_text(&anchor.label.text, chars_for_width(label_rect.width)),
        },
        badges: place_badges(
            &anchor.badges,
            rect.x + 8,
            label_rect.bottom() + 4,
            rect.width - 16,
        ),
    }
}

fn measure_anchor(anchor: &VizEdgeAnchor) -> (i32, i32) {
    let label_size = measure_label(&anchor.label.text, 18, 180);
    let widest_badge = anchor
        .badges
        .iter()
        .map(|badge| badge_width(&badge.label))
        .max()
        .unwrap_or_default();
    let width = label_size.0.max(widest_badge + 16).clamp(88, 220);
    let rows = badge_row_count(&anchor.badges, width - 16);
    let height = 12 + label_size.1 + if rows == 0 { 0 } else { 4 + rows * 22 };
    (width, height.max(34))
}

fn build_anchor_lookup<'a>(
    scene: &'a VizScene,
    rects: &BTreeMap<String, VizRect>,
) -> Result<BTreeMap<(&'a str, &'a str), VizRect>, VizError> {
    let mut anchors = BTreeMap::new();
    for edge in &scene.edges {
        if let Some(anchor) = &edge.anchor {
            let work_id = anchor_work_id(&edge.id, &anchor.id);
            let rect = rects
                .get(&work_id)
                .copied()
                .ok_or_else(|| VizError::Layout(format!("layout omitted edge anchor {work_id}")))?;
            anchors.insert((edge.id.as_str(), anchor.id.as_str()), rect);
        }
    }
    Ok(anchors)
}

fn endpoint_point(
    endpoint: &VizEndpoint,
    nodes: &BTreeMap<&str, &VizLayoutNode>,
    anchors: &BTreeMap<(&str, &str), VizRect>,
) -> Result<VizPoint, VizError> {
    match endpoint {
        VizEndpoint::NodePort { node, port } => nodes
            .get(node.as_str())
            .and_then(|value| value.ports.iter().find(|candidate| candidate.id == *port))
            .map(|value| value.point)
            .ok_or_else(|| VizError::Layout(format!("missing layout endpoint {node}.{port}"))),
        VizEndpoint::EdgeAnchor { edge, anchor } => anchors
            .get(&(edge.as_str(), anchor.as_str()))
            .copied()
            .map(VizRect::center)
            .ok_or_else(|| VizError::Layout(format!("missing layout edge anchor {edge}.{anchor}"))),
    }
}

fn port_point(kind: VizPortKind, rect: VizRect) -> VizPoint {
    match kind {
        VizPortKind::In => VizPoint {
            x: rect.x,
            y: rect.y + rect.height / 2,
        },
        VizPortKind::Out | VizPortKind::Relation => VizPoint {
            x: rect.right(),
            y: rect.y + rect.height / 2,
        },
        VizPortKind::Subject => VizPoint {
            x: rect.x,
            y: rect.y + rect.height / 4,
        },
        VizPortKind::Predicate => VizPoint {
            x: rect.x,
            y: rect.y + rect.height / 2,
        },
        VizPortKind::Object => VizPoint {
            x: rect.x,
            y: rect.y + rect.height * 3 / 4,
        },
    }
}

fn route_edge(
    start: VizPoint,
    end: VizPoint,
    required: &[VizPoint],
    parallel_offset: i32,
) -> Vec<VizPoint> {
    if start == end {
        return vec![
            start,
            VizPoint {
                x: start.x + 54 + parallel_offset.abs(),
                y: start.y,
            },
            VizPoint {
                x: start.x + 54 + parallel_offset.abs(),
                y: start.y - 54 - parallel_offset,
            },
            VizPoint {
                x: start.x,
                y: start.y - 54 - parallel_offset,
            },
            start,
        ];
    }
    let mut checkpoints = Vec::with_capacity(required.len() + 2);
    checkpoints.push(start);
    checkpoints.extend(required.iter().copied());
    checkpoints.push(end);
    let mut points = Vec::new();
    for pair in checkpoints.windows(2) {
        append_orthogonal_segment(&mut points, pair[0], pair[1], parallel_offset);
    }
    dedup_points(&mut points);
    points
}

fn append_orthogonal_segment(
    points: &mut Vec<VizPoint>,
    start: VizPoint,
    end: VizPoint,
    offset: i32,
) {
    if points.last().copied() != Some(start) {
        points.push(start);
    }
    if start.x <= end.x {
        let middle_x = start.x + (end.x - start.x) / 2 + offset;
        points.push(VizPoint {
            x: middle_x,
            y: start.y,
        });
        points.push(VizPoint {
            x: middle_x,
            y: end.y,
        });
    } else {
        let detour_y = start.y.min(end.y) - 48 - offset.abs();
        points.push(VizPoint {
            x: start.x + 28 + offset,
            y: start.y,
        });
        points.push(VizPoint {
            x: start.x + 28 + offset,
            y: detour_y,
        });
        points.push(VizPoint {
            x: end.x - 28 + offset,
            y: detour_y,
        });
        points.push(VizPoint {
            x: end.x - 28 + offset,
            y: end.y,
        });
    }
    points.push(end);
}

fn dedup_points(points: &mut Vec<VizPoint>) {
    points.dedup();
    let mut index = 1;
    while index + 1 < points.len() {
        let previous = points[index - 1];
        let current = points[index];
        let next = points[index + 1];
        if (previous.x == current.x && current.x == next.x)
            || (previous.y == current.y && current.y == next.y)
        {
            points.remove(index);
        } else {
            index += 1;
        }
    }
}

fn parallel_edge_offsets(scene: &VizScene) -> BTreeMap<String, i32> {
    let mut groups: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for edge in &scene.edges {
        let source = endpoint_work_id(&edge.source);
        let target = endpoint_work_id(&edge.target);
        groups
            .entry((source, target))
            .or_default()
            .push(edge.id.clone());
    }
    let mut offsets = BTreeMap::new();
    for edges in groups.values_mut() {
        edges.sort();
        let center = i32_from_usize(edges.len().saturating_sub(1)) * 7;
        for (index, edge) in edges.iter().enumerate() {
            offsets.insert(edge.clone(), i32_from_usize(index) * 14 - center);
        }
    }
    offsets
}

fn place_edge_label(points: &[VizPoint], size: (i32, i32), occupied: &[VizRect]) -> VizRect {
    let (start, end) = longest_segment(points).unwrap_or_else(|| {
        (
            points.first().copied().unwrap_or_default(),
            points.last().copied().unwrap_or_default(),
        )
    });
    let center = VizPoint {
        x: i32::midpoint(start.x, end.x),
        y: i32::midpoint(start.y, end.y),
    };
    for step in 0..16 {
        let offset = if step == 0 {
            0
        } else if step % 2 == 1 {
            -24 * ((step + 1) / 2)
        } else {
            24 * (step / 2)
        };
        let candidate = VizRect {
            x: center.x - size.0 / 2,
            y: center.y - size.1 / 2 + offset,
            width: size.0,
            height: size.1,
        };
        if !occupied.iter().any(|rect| candidate.intersects(*rect)) {
            return candidate;
        }
    }
    VizRect {
        x: center.x - size.0 / 2,
        y: center.y - size.1 / 2 - 408,
        width: size.0,
        height: size.1,
    }
}

fn longest_segment(points: &[VizPoint]) -> Option<(VizPoint, VizPoint)> {
    points
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .max_by_key(|(start, end)| (start.x - end.x).abs() + (start.y - end.y).abs())
}

fn place_badges(
    badges: &[VizBadge],
    start_x: i32,
    start_y: i32,
    max_width: i32,
) -> Vec<VizLayoutBadge> {
    let mut out = Vec::with_capacity(badges.len());
    let mut x = start_x;
    let mut y = start_y;
    for (index, badge) in badges.iter().enumerate() {
        let width = badge_width(&badge.label).min(max_width.max(40));
        if x > start_x && x + width > start_x + max_width {
            x = start_x;
            y += 22;
        }
        out.push(VizLayoutBadge {
            index,
            rect: VizRect {
                x,
                y,
                width,
                height: 18,
            },
        });
        x += width + 6;
    }
    out
}

fn badge_row_count(badges: &[VizBadge], max_width: i32) -> i32 {
    if badges.is_empty() {
        return 0;
    }
    let mut rows = 1;
    let mut used = 0;
    for badge in badges {
        let width = badge_width(&badge.label).min(max_width.max(40));
        if used > 0 && used + 6 + width > max_width {
            rows += 1;
            used = width;
        } else {
            used += usize_to_i32(usize::from(used > 0)) * 6 + width;
        }
    }
    rows
}

fn badge_width(label: &str) -> i32 {
    (i32_from_usize(label.chars().count()) * 7 + 18).clamp(42, 180)
}

fn measure_label(value: &str, max_chars: usize, max_width: i32) -> (i32, i32) {
    let lines = wrap_text(value, max_chars);
    let width = lines
        .iter()
        .map(|line| i32_from_usize(line.chars().count()) * CHAR_WIDTH + 16)
        .max()
        .unwrap_or(32)
        .min(max_width)
        .max(32);
    (width, LINE_HEIGHT * i32_from_usize(lines.len()) + 8)
}

fn wrap_text(value: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    if value.chars().count() <= max_chars {
        return vec![value.to_owned()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let needed = word.chars().count() + usize::from(!current.is_empty());
        if current.chars().count() + needed > max_chars && !current.is_empty() {
            lines.push(current);
            current = String::new();
        }
        if word.chars().count() > max_chars {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
            }
            let chars = word.chars().collect::<Vec<_>>();
            for chunk in chars.chunks(max_chars) {
                lines.push(chunk.iter().collect());
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        vec![value.chars().take(max_chars).collect()]
    } else {
        lines
    }
}

fn chars_for_width(width: i32) -> usize {
    usize::try_from((width / CHAR_WIDTH).max(1)).unwrap_or(1)
}

fn layout_table_scene(scene: &VizScene, options: &VizLayoutOptions) -> Result<VizLayout, VizError> {
    let table = scene
        .table
        .as_ref()
        .ok_or_else(|| VizError::Layout("table scene is missing table records".to_owned()))?;
    let column_count = table.fields.len();
    let mut widths = vec![96; column_count];
    for (column, field) in table.fields.iter().enumerate() {
        widths[column] = widths[column].max(field_label(*field).len() as i32 * CHAR_WIDTH + 24);
        for row in &table.rows {
            if let Some(cell) = row.cells.get(column) {
                widths[column] = widths[column]
                    .max(i32_from_usize(cell.text.chars().count()) * CHAR_WIDTH + 24)
                    .min(320);
            }
        }
    }
    let mut row_heights = vec![42; table.rows.len() + 1];
    for (row_index, row) in table.rows.iter().enumerate() {
        let line_count = row
            .cells
            .iter()
            .enumerate()
            .map(|(column, cell)| wrap_text(&cell.text, chars_for_width(widths[column] - 20)).len())
            .max()
            .unwrap_or(1);
        row_heights[row_index + 1] = (i32_from_usize(line_count) * LINE_HEIGHT + 20).max(42);
    }
    let table_width = widths.iter().sum();
    let table_height = row_heights.iter().sum();
    let table_rect = VizRect {
        x: options.margin,
        y: options.margin,
        width: table_width,
        height: table_height,
    };
    let mut cells = Vec::new();
    let mut x = options.margin;
    for (column, field) in table.fields.iter().enumerate() {
        let rect = VizRect {
            x,
            y: options.margin,
            width: widths[column],
            height: row_heights[0],
        };
        cells.push(table_cell(0, column, rect, field_label(*field)));
        x += widths[column];
    }
    let mut y = options.margin + row_heights[0];
    for (row_index, row) in table.rows.iter().enumerate() {
        x = options.margin;
        for (column, cell) in row.cells.iter().enumerate() {
            let rect = VizRect {
                x,
                y,
                width: widths[column],
                height: row_heights[row_index + 1],
            };
            cells.push(table_cell(row_index + 1, column, rect, &cell.text));
            x += widths[column];
        }
        y += row_heights[row_index + 1];
    }
    Ok(VizLayout {
        schema_version: VIZ_LAYOUT_SCHEMA_VERSION.to_owned(),
        mode: VizMode::Table,
        width: table_rect.right() + options.margin,
        height: table_rect.bottom() + options.margin,
        nodes: Vec::new(),
        edges: Vec::new(),
        table: Some(VizLayoutTable {
            rect: table_rect,
            cells,
        }),
    })
}

fn table_cell(row: usize, column: usize, rect: VizRect, text: &str) -> VizLayoutTableCell {
    let label_rect = VizRect {
        x: rect.x + 10,
        y: rect.y + 10,
        width: rect.width - 20,
        height: rect.height - 20,
    };
    VizLayoutTableCell {
        row,
        column,
        rect,
        label: VizLayoutLabel {
            rect: label_rect,
            lines: wrap_text(text, chars_for_width(label_rect.width)),
        },
    }
}

fn field_label(field: VizTableField) -> &'static str {
    match field {
        VizTableField::Statement => "Statement",
        VizTableField::AssertedIn => "Asserted in",
        VizTableField::Reifiers => "Reifiers",
        VizTableField::Annotations => "Annotations",
        VizTableField::ReferencedBy => "Referenced by",
        VizTableField::Depth => "Depth",
        VizTableField::Diagnostics => "Diagnostics",
    }
}

/// Validate geometry and scene-to-layout completeness.
pub fn validate_layout(scene: &VizScene, layout: &VizLayout) -> Result<(), VizError> {
    if layout.width <= 0 || layout.height <= 0 {
        return Err(VizError::Layout(
            "visualization layout has non-positive canvas bounds".to_owned(),
        ));
    }
    if scene.mode == VizMode::Table {
        let Some(table) = &layout.table else {
            return Err(VizError::Layout(
                "table scene has no table geometry".to_owned(),
            ));
        };
        if table.cells.len()
            != scene
                .table
                .as_ref()
                .map_or(0, |value| value.fields.len() * (value.rows.len() + 1))
        {
            return Err(VizError::Layout(
                "table geometry does not cover every scene cell".to_owned(),
            ));
        }
        return validate_label_bounds(table.cells.iter().map(|cell| (&cell.label, cell.rect)));
    }
    let nodes = layout
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    if nodes.len() != scene.nodes.len() || layout.edges.len() != scene.edges.len() {
        return Err(VizError::Layout(
            "graph layout does not cover every scene node and edge".to_owned(),
        ));
    }
    for scene_node in &scene.nodes {
        if !nodes.contains_key(scene_node.id.as_str()) {
            return Err(VizError::Layout(format!(
                "graph layout omitted scene node {}",
                scene_node.id
            )));
        }
    }
    for (index, left) in layout.nodes.iter().enumerate() {
        for right in layout.nodes.iter().skip(index + 1) {
            if left.rect.intersects(right.rect) {
                return Err(VizError::Layout(format!(
                    "layout nodes {} and {} overlap",
                    left.id, right.id
                )));
            }
        }
    }
    validate_label_bounds(layout.nodes.iter().map(|node| (&node.label, node.rect)))?;
    for edge in &layout.edges {
        if edge.points.len() < 2 {
            return Err(VizError::Layout(format!(
                "layout edge {} has fewer than two route points",
                edge.id
            )));
        }
        if edge
            .points
            .windows(2)
            .any(|pair| pair[0].x != pair[1].x && pair[0].y != pair[1].y)
        {
            return Err(VizError::Layout(format!(
                "layout edge {} contains a non-orthogonal segment",
                edge.id
            )));
        }
    }
    Ok(())
}

fn validate_label_bounds<'a>(
    labels: impl Iterator<Item = (&'a VizLayoutLabel, VizRect)>,
) -> Result<(), VizError> {
    for (label, parent) in labels {
        if label.rect.x < parent.x
            || label.rect.y < parent.y
            || label.rect.right() > parent.right()
            || label.rect.bottom() > parent.bottom()
        {
            return Err(VizError::Layout(
                "layout label lies outside its containing box".to_owned(),
            ));
        }
        if label
            .lines
            .iter()
            .any(|line| i32_from_usize(line.chars().count()) * CHAR_WIDTH > label.rect.width)
        {
            return Err(VizError::Layout(
                "layout label text exceeds its assigned box".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_options(options: &VizLayoutOptions) -> Result<(), VizError> {
    if options.margin < 0
        || options.rank_spacing < 32
        || options.node_spacing < 16
        || options.component_spacing < 16
        || options.component_wrap_width < 320
        || options.crossing_sweeps == 0
        || options.max_node_width < 160
    {
        return Err(VizError::Layout(
            "visualization layout options violate deterministic geometry bounds".to_owned(),
        ));
    }
    Ok(())
}

fn i32_from_usize(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX / 4)
}

fn usize_to_i32(value: usize) -> i32 {
    i32_from_usize(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EX: &str = "https://example.org/";

    fn iri(local: &str) -> TermValue {
        TermValue::Iri(format!("{EX}{local}"))
    }

    fn graph_input() -> VizGraphInput {
        let mut quads = Vec::new();
        for (source, predicate, target) in [
            ("alice", "knows", "bob"),
            ("alice", "knows", "carol"),
            ("bob", "reportsTo", "dana"),
            ("carol", "reportsTo", "dana"),
            ("dana", "knows", "alice"),
        ] {
            quads.push(VizInputQuad {
                subject: iri(source),
                predicate: format!("{EX}{predicate}"),
                object: iri(target),
                graph_name: Some(iri("facts")),
            });
        }
        VizGraphInput {
            quads,
            reifiers: vec![VizInputReifier {
                reifier: iri("claim"),
                statement: VizInputStatement {
                    subject: iri("alice"),
                    predicate: format!("{EX}knows"),
                    object: iri("bob"),
                },
                graph_name: Some(iri("claims")),
            }],
            annotations: vec![VizInputAnnotation {
                reifier: iri("claim"),
                predicate: format!("{EX}confidence"),
                object: TermValue::Literal {
                    lexical_form: "0.82".to_owned(),
                    datatype: "http://www.w3.org/2001/XMLSchema#decimal".to_owned(),
                    language: None,
                    direction: None,
                },
                graph_name: Some(iri("provenance")),
            }],
        }
    }

    #[test]
    fn compact_layout_is_connected_non_overlapping_and_deterministic() {
        let (_, scene) =
            project_graph_input_scene(&graph_input(), &VizSpec::default()).expect("scene");
        let options = VizLayoutOptions::default();
        let first = layout_scene(&scene, &options).expect("layout");
        let second = layout_scene(&scene, &options).expect("layout");
        assert_eq!(first, second);
        assert_eq!(first.nodes.len(), scene.nodes.len());
        assert_eq!(first.edges.len(), scene.edges.len());
        assert!(first.edges.iter().any(|edge| edge.anchor.is_some()));
    }

    #[test]
    fn incidence_layout_uses_multiple_ranks_and_orthogonal_routes() {
        let spec = VizSpec {
            mode: VizMode::Incidence,
            ..VizSpec::default()
        };
        let (_, scene) = project_graph_input_scene(&graph_input(), &spec).expect("scene");
        let layout = layout_scene(&scene, &VizLayoutOptions::default()).expect("layout");
        assert!(
            layout
                .nodes
                .iter()
                .map(|node| node.rank)
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        );
        assert!(layout.edges.iter().all(|edge| {
            edge.points
                .windows(2)
                .all(|pair| pair[0].x == pair[1].x || pair[0].y == pair[1].y)
        }));
    }

    #[test]
    fn table_layout_contains_every_header_and_cell() {
        let spec = VizSpec {
            mode: VizMode::Table,
            table_fields: vec![
                VizTableField::Statement,
                VizTableField::AssertedIn,
                VizTableField::Reifiers,
            ],
            ..VizSpec::default()
        };
        let (_, scene) = project_graph_input_scene(&graph_input(), &spec).expect("scene");
        let layout = layout_scene(&scene, &VizLayoutOptions::default()).expect("layout");
        let table = layout.table.expect("table");
        assert_eq!(
            table.cells.len(),
            3 * (scene.table.expect("scene table").rows.len() + 1)
        );
    }

    #[test]
    fn invalid_layout_options_hard_error() {
        let (_, scene) =
            project_graph_input_scene(&graph_input(), &VizSpec::default()).expect("scene");
        let options = VizLayoutOptions {
            crossing_sweeps: 0,
            ..VizLayoutOptions::default()
        };
        assert!(matches!(
            layout_scene(&scene, &options),
            Err(VizError::Layout(_))
        ));
    }
}
