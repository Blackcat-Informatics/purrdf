// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original identities and geometry at deterministic layout refusal.

use super::{VizLayoutOptions, VizPoint, VizRect};
use purrdf_hash::error::{ErrorValue, StructuredError};
use std::{fmt, num::TryFromIntError};

/// The checked self-edge arithmetic operation that refused its original operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VizLayoutArithmetic {
    /// Twice the routing padding.
    Padding,
    /// Width plus twice the padding.
    Width,
    /// Height plus twice the padding.
    Height,
    /// Width minus twice the padding, requiring a positive result.
    InsetWidth,
    /// Height minus twice the padding, requiring a positive result.
    InsetHeight,
    /// Horizontal coordinate plus padding.
    X,
    /// Vertical coordinate plus padding.
    Y,
}

/// A native layout refusal with original identities and comparison operands.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VizLayoutError {
    /// A scene node has no rectangle.
    MissingSceneNode {
        /// Original node identity.
        node: String,
    },
    /// A scene node has no rank.
    MissingRank {
        /// Original node identity.
        node: String,
    },
    /// An edge endpoint is absent from the work graph.
    MissingWorkEndpoint {
        /// Original edge identity.
        edge: String,
        /// Original source work identity.
        source: String,
        /// Original target work identity.
        target: String,
    },
    /// Cycle breaking left unvisited component nodes.
    Cycle {
        /// Visited node count.
        visited: usize,
        /// Expected node count.
        expected: usize,
    },
    /// The computed work anchor has no rectangle.
    MissingAnchor {
        /// Original work identity.
        work_id: String,
        /// Original edge identity.
        edge: String,
        /// Original anchor identity.
        anchor: String,
    },
    /// A node port has no geometry.
    MissingEndpoint {
        /// Original node identity.
        node: String,
        /// Original port identity.
        port: String,
    },
    /// An edge anchor has no geometry.
    MissingEdgeAnchor {
        /// Original edge identity.
        edge: String,
        /// Original anchor identity.
        anchor: String,
    },
    /// Checked arithmetic overflowed or consumed a positive dimension.
    Arithmetic {
        /// Refused operation.
        operation: VizLayoutArithmetic,
        /// Original left operand.
        left: i32,
        /// Original right operand.
        right: i32,
    },
    /// A table-mode scene has no table records.
    MissingTableRecords,
    /// The canvas has a non-positive dimension.
    CanvasBounds {
        /// Original width.
        width: i32,
        /// Original height.
        height: i32,
    },
    /// Legend geometry does not cover the scene.
    LegendCoverage {
        /// Actual entry count.
        actual: usize,
        /// Expected entry count.
        expected: usize,
    },
    /// A table scene has no table geometry.
    MissingTableGeometry,
    /// Table geometry does not cover the scene cells.
    TableCoverage {
        /// Actual cell count.
        actual: usize,
        /// Expected cell count.
        expected: usize,
    },
    /// Graph geometry does not cover the scene nodes and edges.
    GraphCoverage {
        /// Distinct layout node count.
        nodes: usize,
        /// Scene node count.
        expected_nodes: usize,
        /// Layout edge count.
        edges: usize,
        /// Scene edge count.
        expected_edges: usize,
    },
    /// A scene node is missing after geometry coverage checks.
    GraphNodeMissing {
        /// Original node identity.
        node: String,
    },
    /// Two node rectangles intersect.
    NodeOverlap {
        /// Original left identity.
        left: String,
        /// Original right identity.
        right: String,
        /// Original left rectangle.
        left_rect: VizRect,
        /// Original right rectangle.
        right_rect: VizRect,
    },
    /// An edge has fewer than two route points.
    RoutePoints {
        /// Original edge identity.
        edge: String,
        /// Original ordered points.
        points: Vec<VizPoint>,
    },
    /// An edge contains a non-orthogonal segment.
    RouteOrthogonality {
        /// Original edge identity.
        edge: String,
        /// Original ordered points.
        points: Vec<VizPoint>,
    },
    /// A label rectangle exceeds its parent.
    LabelBounds {
        /// Original label rectangle.
        rect: VizRect,
        /// Original containing rectangle.
        parent: VizRect,
    },
    /// Label text exceeds its rectangle's width.
    LabelWidth {
        /// Original label rectangle.
        rect: VizRect,
        /// Original ordered lines.
        lines: Vec<String>,
    },
    /// Layout options violate geometry bounds.
    Options {
        /// Original requested options.
        options: VizLayoutOptions,
    },
    /// The coordinate conversion refused the original value.
    Coordinate {
        /// Original coordinate.
        value: i64,
        /// Original conversion cause.
        cause: TryFromIntError,
    },
    /// A child scope has no component declaration.
    MissingComponentDeclaration {
        /// Original child identity.
        component: String,
    },
    /// Component geometry lost its declaration.
    ComponentDeclarationLost {
        /// Original geometry identity.
        component: String,
    },
    /// A component node has no rank.
    ComponentRankMissing {
        /// Original node identity.
        node: String,
    },
    /// The anchor ancestry unexpectedly became empty.
    EmptyAnchorAncestry {
        /// Original edge identity.
        edge: String,
        /// Original anchor identity.
        anchor: String,
    },
    /// Anchor containment failed to stabilize within the existing iteration bound.
    UnstableAnchorContainment {
        /// Original edge count defining the bound.
        edges: usize,
    },
    /// A compound edge source has no rectangle.
    MissingCompoundSource {
        /// Original edge identity.
        edge: String,
        /// Original source work identity.
        source: String,
    },
    /// A compound edge target has no rectangle.
    MissingCompoundTarget {
        /// Original edge identity.
        edge: String,
        /// Original target work identity.
        target: String,
    },
    /// Region nesting depth cannot be represented.
    RegionDepth {
        /// Original region identity.
        region: String,
        /// Original nesting depth.
        depth: usize,
        /// Original conversion cause.
        cause: TryFromIntError,
    },
    /// A scenario interaction predicate is absent.
    MissingScenarioPredicate {
        /// Original interaction identity.
        interaction: String,
        /// Original predicate identity.
        predicate: String,
    },
    /// A region frame lost its identity prefix.
    RegionIdentityLost {
        /// Original frame identity.
        frame: String,
    },
    /// A presentation rectangle is empty or outside the canvas.
    PresentationRectangle {
        /// Original identity.
        id: String,
        /// Original rectangle.
        rect: VizRect,
        /// Original canvas.
        canvas: VizRect,
    },
    /// Presentation label lines exceed the assigned height.
    LabelHeight {
        /// Original label rectangle.
        rect: VizRect,
        /// Original line count.
        lines: usize,
        /// Native line height.
        line_height: i32,
    },
    /// Presentation geometry does not cover every instruction.
    InstructionCoverage {
        /// Actual component count.
        components: usize,
        /// Expected component count.
        expected_components: usize,
        /// Actual participant count.
        participants: usize,
        /// Expected participant count.
        expected_participants: usize,
        /// Actual region count.
        regions: usize,
        /// Expected region count.
        expected_regions: usize,
        /// Actual interaction count.
        interactions: usize,
        /// Expected interaction count.
        expected_interactions: usize,
        /// Actual detail-heading presence.
        detail_heading: bool,
        /// Expected detail-heading presence.
        expected_detail_heading: bool,
    },
    /// Presentation geometry repeats a frame identity.
    DuplicateFrames {
        /// Distinct frame count.
        unique: usize,
        /// Original component frame count.
        components: usize,
        /// Original region frame count.
        regions: usize,
    },
    /// A frame's declared parent geometry is absent.
    MissingParentFrame {
        /// Original frame identity.
        frame: String,
        /// Original declared parent identity.
        parent: String,
    },
    /// A frame leaves its parent boundary.
    FrameBounds {
        /// Original frame identity.
        frame: String,
        /// Original parent identity.
        parent: String,
        /// Original frame rectangle.
        rect: VizRect,
        /// Original parent rectangle.
        parent_rect: VizRect,
    },
    /// An admitted component has no frame.
    MissingComponentFrame {
        /// Original component identity.
        component: String,
    },
    /// A component's scene node has no geometry.
    MissingComponentNode {
        /// Original component identity.
        component: String,
        /// Original scene node identity.
        node: String,
    },
    /// A component frame does not contain a declared node.
    ComponentBounds {
        /// Original component identity.
        component: String,
        /// Original node identity.
        node: String,
        /// Original frame rectangle.
        frame: VizRect,
        /// Original node rectangle.
        rect: VizRect,
    },
    /// A boundary port has no frame.
    MissingPortFrame {
        /// Original port identity.
        port: String,
        /// Original boundary identity.
        boundary: String,
    },
    /// A boundary port has no native RDF edge.
    MissingPortEdge {
        /// Original port identity.
        port: String,
        /// Original edge identity.
        edge: String,
    },
    /// A boundary port is outside its perimeter or actual route.
    PortRoute {
        /// Original port identity.
        port: String,
        /// Original boundary identity.
        boundary: String,
        /// Original edge identity.
        edge: String,
        /// Original port point.
        point: VizPoint,
        /// Original frame rectangle.
        frame: VizRect,
        /// Original ordered edge route.
        points: Vec<VizPoint>,
    },
    /// A participant has an invalid lifeline.
    Lifeline {
        /// Original participant identity.
        participant: String,
        /// Original start point.
        start: VizPoint,
        /// Original end point.
        end: VizPoint,
        /// Original canvas.
        canvas: VizRect,
    },
    /// An interaction has an invalid orthogonal route.
    InteractionRoute {
        /// Original interaction identity.
        interaction: String,
        /// Original ordered points.
        points: Vec<VizPoint>,
        /// Original canvas.
        canvas: VizRect,
    },
}

impl fmt::Display for VizLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSceneNode { node } => write!(f, "layout omitted scene node {node}"),
            Self::MissingRank { node } => write!(f, "layout omitted rank for scene node {node}"),
            Self::MissingWorkEndpoint { edge, .. } => write!(
                f,
                "scene edge {edge} has an endpoint absent from the work graph"
            ),
            Self::Cycle { .. } => f.write_str("deterministic cycle breaking did not produce a DAG"),
            Self::MissingAnchor { work_id, .. } => {
                write!(f, "layout omitted edge anchor {work_id}")
            }
            Self::MissingEndpoint { node, port } => {
                write!(f, "missing layout endpoint {node}.{port}")
            }
            Self::MissingEdgeAnchor { edge, anchor } => {
                write!(f, "missing layout edge anchor {edge}.{anchor}")
            }
            Self::Arithmetic { operation, .. } => f.write_str(match operation {
                VizLayoutArithmetic::Padding => "self-edge padding overflow",
                VizLayoutArithmetic::Width => "self-edge width overflow",
                VizLayoutArithmetic::Height => "self-edge height overflow",
                VizLayoutArithmetic::InsetWidth => "self-edge padding consumed its width",
                VizLayoutArithmetic::InsetHeight => "self-edge padding consumed its height",
                VizLayoutArithmetic::X => "self-edge x overflow",
                VizLayoutArithmetic::Y => "self-edge y overflow",
            }),
            Self::MissingTableRecords => f.write_str("table scene is missing table records"),
            Self::CanvasBounds { .. } => {
                f.write_str("visualization layout has non-positive canvas bounds")
            }
            Self::LegendCoverage { .. } => f.write_str("layout does not cover every legend entry"),
            Self::MissingTableGeometry => f.write_str("table scene has no table geometry"),
            Self::TableCoverage { .. } => {
                f.write_str("table geometry does not cover every scene cell")
            }
            Self::GraphCoverage { .. } => {
                f.write_str("graph layout does not cover every scene node and edge")
            }
            Self::GraphNodeMissing { node } => write!(f, "graph layout omitted scene node {node}"),
            Self::NodeOverlap { left, right, .. } => {
                write!(f, "layout nodes {left} and {right} overlap")
            }
            Self::RoutePoints { edge, .. } => {
                write!(f, "layout edge {edge} has fewer than two route points")
            }
            Self::RouteOrthogonality { edge, .. } => {
                write!(f, "layout edge {edge} contains a non-orthogonal segment")
            }
            Self::LabelBounds { .. } => f.write_str("layout label lies outside its containing box"),
            Self::LabelWidth { .. } => f.write_str("layout label text exceeds its assigned box"),
            Self::Options { .. } => {
                f.write_str("visualization layout options violate deterministic geometry bounds")
            }
            Self::Coordinate { .. } => f.write_str("presentation coordinate cannot be represented"),
            Self::MissingComponentDeclaration { .. } => f.write_str("component declaration absent"),
            Self::ComponentDeclarationLost { .. } => {
                f.write_str("component geometry lost its declaration")
            }
            Self::ComponentRankMissing { node } => {
                write!(f, "component layout omitted rank for {node}")
            }
            Self::EmptyAnchorAncestry { .. } => f.write_str("anchor ancestry became empty"),
            Self::UnstableAnchorContainment { .. } => {
                f.write_str("statement-anchor containment did not reach a stable common ancestor")
            }
            Self::MissingCompoundSource { .. } => f.write_str("missing compound source"),
            Self::MissingCompoundTarget { .. } => f.write_str("missing compound target"),
            Self::RegionDepth { .. } => f.write_str("region depth overflow"),
            Self::MissingScenarioPredicate { .. } => f.write_str("scenario predicate absent"),
            Self::RegionIdentityLost { .. } => f.write_str("region geometry lost its identity"),
            Self::PresentationRectangle { id, .. } => write!(
                f,
                "presentation rectangle {id} is empty or outside the complete canvas"
            ),
            Self::LabelHeight { .. } => {
                f.write_str("presentation label lines exceed their assigned height")
            }
            Self::InstructionCoverage { .. } => {
                f.write_str("presentation geometry does not cover every admitted instruction")
            }
            Self::DuplicateFrames { .. } => {
                f.write_str("presentation geometry repeats a frame identity")
            }
            Self::MissingParentFrame { frame, .. } => {
                write!(f, "presentation frame {frame} has no parent geometry")
            }
            Self::FrameBounds { frame, .. } => {
                write!(f, "presentation frame {frame} leaves its parent boundary")
            }
            Self::MissingComponentFrame { .. } => f.write_str("component frame identity is absent"),
            Self::MissingComponentNode { .. } => f.write_str("component node has no geometry"),
            Self::ComponentBounds {
                component, node, ..
            } => write!(
                f,
                "component {component} does not contain its declared node {node}"
            ),
            Self::MissingPortFrame { .. } => f.write_str("boundary port has no frame"),
            Self::MissingPortEdge { .. } => f.write_str("boundary port has no native RDF edge"),
            Self::PortRoute { port, .. } => write!(
                f,
                "boundary port {port} is not on its perimeter and actual edge route"
            ),
            Self::Lifeline { participant, .. } => {
                write!(f, "participant {participant} has an invalid lifeline")
            }
            Self::InteractionRoute { interaction, .. } => write!(
                f,
                "interaction {interaction} has an invalid orthogonal route"
            ),
        }
    }
}

impl std::error::Error for VizLayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Coordinate { cause, .. } | Self::RegionDepth { cause, .. } => Some(cause),
            _ => None,
        }
    }
}

impl StructuredError for VizRect {
    purrdf_hash::error_visit! { @record self { x, y, width, height } => "rdf-viz-rectangle" {
        "x" => Signed(self.x.into()), "y" => Signed(self.y.into()),
        "width" => Signed(self.width.into()), "height" => Signed(self.height.into()),
    } }
}
impl StructuredError for VizPoint {
    purrdf_hash::error_visit! { @record self { x, y } => "rdf-viz-point" {
        "x" => Signed(self.x.into()), "y" => Signed(self.y.into()),
    } }
}
impl StructuredError for VizLayoutOptions {
    purrdf_hash::error_visit! { @record self { margin, rank_spacing, node_spacing, component_spacing, component_wrap_width, crossing_sweeps, max_node_width } => "rdf-viz-layout-options" {
        "margin" => Signed(self.margin.into()), "rank_spacing" => Signed(self.rank_spacing.into()),
        "node_spacing" => Signed(self.node_spacing.into()), "component_spacing" => Signed(self.component_spacing.into()),
        "component_wrap_width" => Signed(self.component_wrap_width.into()), "crossing_sweeps" => Unsigned(self.crossing_sweeps.into()),
        "max_node_width" => Signed(self.max_node_width.into()),
    } }
}

impl StructuredError for VizLayoutError {
    purrdf_hash::error_visit! {
        MissingSceneNode { node } => "rdf-viz-layout-missing-scene-node" { "node" => Text(node) },
        MissingRank { node } => "rdf-viz-layout-missing-rank" { "node" => Text(node) },
        MissingWorkEndpoint { edge, source, target } => "rdf-viz-layout-missing-work-endpoint" { "edge" => Text(edge), "source" => Text(source), "target" => Text(target) },
        Cycle { visited, expected } => "rdf-viz-layout-cycle" { "visited" => Unsigned(*visited as u128), "expected" => Unsigned(*expected as u128) },
        MissingAnchor { work_id, edge, anchor } => "rdf-viz-layout-missing-anchor" { "work_id" => Text(work_id), "edge" => Text(edge), "anchor" => Text(anchor) },
        MissingEndpoint { node, port } => "rdf-viz-layout-missing-endpoint" { "node" => Text(node), "port" => Text(port) },
        MissingEdgeAnchor { edge, anchor } => "rdf-viz-layout-missing-edge-anchor" { "edge" => Text(edge), "anchor" => Text(anchor) },
        Arithmetic { operation, left, right } => "rdf-viz-layout-arithmetic" {
            "operation" => Code(match operation { VizLayoutArithmetic::Padding => "padding", VizLayoutArithmetic::Width => "width", VizLayoutArithmetic::Height => "height", VizLayoutArithmetic::InsetWidth => "inset-width", VizLayoutArithmetic::InsetHeight => "inset-height", VizLayoutArithmetic::X => "x", VizLayoutArithmetic::Y => "y" }),
            "left" => Signed((*left).into()), "right" => Signed((*right).into()),
        },
        MissingTableRecords => "rdf-viz-layout-missing-table-records" {},
        CanvasBounds { width, height } => "rdf-viz-layout-canvas-bounds" { "width" => Signed((*width).into()), "height" => Signed((*height).into()) },
        LegendCoverage { actual, expected } => "rdf-viz-layout-legend-coverage" { "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128) },
        MissingTableGeometry => "rdf-viz-layout-missing-table-geometry" {},
        TableCoverage { actual, expected } => "rdf-viz-layout-table-coverage" { "actual" => Unsigned(*actual as u128), "expected" => Unsigned(*expected as u128) },
        GraphCoverage { nodes, expected_nodes, edges, expected_edges } => "rdf-viz-layout-graph-coverage" { "nodes" => Unsigned(*nodes as u128), "expected_nodes" => Unsigned(*expected_nodes as u128), "edges" => Unsigned(*edges as u128), "expected_edges" => Unsigned(*expected_edges as u128) },
        GraphNodeMissing { node } => "rdf-viz-layout-graph-node-missing" { "node" => Text(node) },
        NodeOverlap { left, right, left_rect, right_rect } => "rdf-viz-layout-node-overlap" { "left" => Text(left), "right" => Text(right), "left_rect" => Record(left_rect), "right_rect" => Record(right_rect) },
        RoutePoints { edge, points } => "rdf-viz-layout-route-points" { "edge" => Text(edge), "points" => List(points.iter().map(|point| ErrorValue::Record(point))) },
        RouteOrthogonality { edge, points } => "rdf-viz-layout-route-orthogonality" { "edge" => Text(edge), "points" => List(points.iter().map(|point| ErrorValue::Record(point))) },
        LabelBounds { rect, parent } => "rdf-viz-layout-label-bounds" { "rect" => Record(rect), "parent" => Record(parent) },
        LabelWidth { rect, lines } => "rdf-viz-layout-label-width" { "rect" => Record(rect), "lines" => List(lines.iter().map(|line| ErrorValue::Text(line))) },
        Options { options } => "rdf-viz-layout-invalid-options" { "options" => Record(options) },
        Coordinate { value, cause } => "rdf-viz-layout-coordinate" { "value" => Signed((*value).into()), "cause" => Cause(cause) },
        MissingComponentDeclaration { component } => "rdf-viz-layout-missing-component-declaration" { "component" => Text(component) },
        ComponentDeclarationLost { component } => "rdf-viz-layout-component-declaration-lost" { "component" => Text(component) },
        ComponentRankMissing { node } => "rdf-viz-layout-component-rank-missing" { "node" => Text(node) },
        EmptyAnchorAncestry { edge, anchor } => "rdf-viz-layout-empty-anchor-ancestry" { "edge" => Text(edge), "anchor" => Text(anchor) },
        UnstableAnchorContainment { edges } => "rdf-viz-layout-unstable-anchor-containment" { "edges" => Unsigned(*edges as u128) },
        MissingCompoundSource { edge, source } => "rdf-viz-layout-missing-compound-source" { "edge" => Text(edge), "source" => Text(source) },
        MissingCompoundTarget { edge, target } => "rdf-viz-layout-missing-compound-target" { "edge" => Text(edge), "target" => Text(target) },
        RegionDepth { region, depth, cause } => "rdf-viz-layout-region-depth" { "region" => Text(region), "depth" => Unsigned(*depth as u128), "cause" => Cause(cause) },
        MissingScenarioPredicate { interaction, predicate } => "rdf-viz-layout-missing-scenario-predicate" { "interaction" => Text(interaction), "predicate" => Text(predicate) },
        RegionIdentityLost { frame } => "rdf-viz-layout-region-identity-lost" { "frame" => Text(frame) },
        PresentationRectangle { id, rect, canvas } => "rdf-viz-layout-presentation-rectangle" { "id" => Text(id), "rect" => Record(rect), "canvas" => Record(canvas) },
        LabelHeight { rect, lines, line_height } => "rdf-viz-layout-label-height" { "rect" => Record(rect), "lines" => Unsigned(*lines as u128), "line_height" => Signed((*line_height).into()) },
        InstructionCoverage { components, expected_components, participants, expected_participants, regions, expected_regions, interactions, expected_interactions, detail_heading, expected_detail_heading } => "rdf-viz-layout-instruction-coverage" {
            "components" => Unsigned(*components as u128), "expected_components" => Unsigned(*expected_components as u128),
            "participants" => Unsigned(*participants as u128), "expected_participants" => Unsigned(*expected_participants as u128),
            "regions" => Unsigned(*regions as u128), "expected_regions" => Unsigned(*expected_regions as u128),
            "interactions" => Unsigned(*interactions as u128), "expected_interactions" => Unsigned(*expected_interactions as u128),
            "detail_heading" => Boolean(*detail_heading), "expected_detail_heading" => Boolean(*expected_detail_heading),
        },
        DuplicateFrames { unique, components, regions } => "rdf-viz-layout-duplicate-frames" { "unique" => Unsigned(*unique as u128), "components" => Unsigned(*components as u128), "regions" => Unsigned(*regions as u128) },
        MissingParentFrame { frame, parent } => "rdf-viz-layout-missing-parent-frame" { "frame" => Text(frame), "parent" => Text(parent) },
        FrameBounds { frame, parent, rect, parent_rect } => "rdf-viz-layout-frame-bounds" { "frame" => Text(frame), "parent" => Text(parent), "rect" => Record(rect), "parent_rect" => Record(parent_rect) },
        MissingComponentFrame { component } => "rdf-viz-layout-missing-component-frame" { "component" => Text(component) },
        MissingComponentNode { component, node } => "rdf-viz-layout-missing-component-node" { "component" => Text(component), "node" => Text(node) },
        ComponentBounds { component, node, frame, rect } => "rdf-viz-layout-component-bounds" { "component" => Text(component), "node" => Text(node), "frame" => Record(frame), "rect" => Record(rect) },
        MissingPortFrame { port, boundary } => "rdf-viz-layout-missing-port-frame" { "port" => Text(port), "boundary" => Text(boundary) },
        MissingPortEdge { port, edge } => "rdf-viz-layout-missing-port-edge" { "port" => Text(port), "edge" => Text(edge) },
        PortRoute { port, boundary, edge, point, frame, points } => "rdf-viz-layout-port-route" { "port" => Text(port), "boundary" => Text(boundary), "edge" => Text(edge), "point" => Record(point), "frame" => Record(frame), "points" => List(points.iter().map(|point| ErrorValue::Record(point))) },
        Lifeline { participant, start, end, canvas } => "rdf-viz-layout-lifeline" { "participant" => Text(participant), "start" => Record(start), "end" => Record(end), "canvas" => Record(canvas) },
        InteractionRoute { interaction, points, canvas } => "rdf-viz-layout-interaction-route" { "interaction" => Text(interaction), "points" => List(points.iter().map(|point| ErrorValue::Record(point))), "canvas" => Record(canvas) },
    }
}
