// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic SVG emission over semantic scenes and concrete layouts.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use super::*;

/// SVG emitter options. Semantic and layout choices live outside this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSvgOptions {
    /// Embed the complete versioned [`VizExport`] JSON in `<metadata>`.
    pub embed_metadata: bool,
    /// Include deterministic first-party CSS.
    pub include_styles: bool,
    /// Accessible document title.
    pub title: String,
}

impl Default for VizSvgOptions {
    fn default() -> Self {
        Self {
            embed_metadata: true,
            include_styles: true,
            title: "RDF 1.2 graph".to_owned(),
        }
    }
}

/// Complete render options with independent layout and SVG controls.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizRenderOptions {
    /// Renderer-neutral deterministic layout options.
    pub layout: VizLayoutOptions,
    /// SVG serialization options.
    pub svg: VizSvgOptions,
}

/// Deterministic SVG paired with its load-bearing structured export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSvgDocument {
    /// Complete SVG XML text.
    pub svg: String,
    /// Complete structured export embedded in the SVG by default.
    pub export: VizExport,
}

/// Build a complete versioned export from an existing semantic projection.
pub fn build_export(
    projection: &VizProjection,
    spec: &VizSpec,
    layout_options: &VizLayoutOptions,
) -> Result<VizExport, VizError> {
    let scene = build_scene(projection, spec)?;
    let layout = layout_scene(&scene, layout_options)?;
    let spec_json =
        serde_json::to_string(spec).map_err(|error| VizError::Serialize(error.to_string()))?;
    let model_json = serde_json::to_string(projection)
        .map_err(|error| VizError::Serialize(error.to_string()))?;
    let scene_json =
        serde_json::to_string(&scene).map_err(|error| VizError::Serialize(error.to_string()))?;
    let element_index = build_element_index(&scene, &layout);
    Ok(VizExport {
        schema_version: VIZ_EXPORT_SCHEMA_VERSION.to_owned(),
        spec: spec.clone(),
        spec_hash: stable_hash_hex(&spec_json),
        model_hash: stable_hash_hex(&model_json),
        scene_hash: stable_hash_hex(&scene_json),
        model: projection.clone(),
        scene,
        layout,
        element_index,
        diagnostics: projection.diagnostics.clone(),
    })
}

/// Project a dataset into a complete structured visualization export.
pub fn project_dataset_export(
    dataset: &RdfDataset,
    spec: &VizSpec,
    layout_options: &VizLayoutOptions,
) -> Result<VizExport, VizError> {
    let projection = project_dataset(dataset, spec)?;
    build_export(&projection, spec, layout_options)
}

/// Project graph-like input into a complete structured visualization export.
pub fn project_graph_input_export(
    input: &VizGraphInput,
    spec: &VizSpec,
    layout_options: &VizLayoutOptions,
) -> Result<VizExport, VizError> {
    let projection = project_graph_input(input, spec)?;
    build_export(&projection, spec, layout_options)
}

/// Serialize a complete visualization export to deterministic JSON.
pub fn export_json(export: &VizExport) -> Result<String, VizError> {
    serde_json::to_string(export).map_err(|error| VizError::Serialize(error.to_string()))
}

/// Render an existing semantic projection to deterministic SVG.
pub fn render_projection_svg(
    projection: &VizProjection,
    spec: &VizSpec,
    options: &VizRenderOptions,
) -> Result<VizSvgDocument, VizError> {
    let export = build_export(projection, spec, &options.layout)?;
    let svg = render_export_svg(&export, &options.svg)?;
    Ok(VizSvgDocument { svg, export })
}

/// Project a dataset and render deterministic SVG.
pub fn render_dataset_svg(
    dataset: &RdfDataset,
    spec: &VizSpec,
    options: &VizRenderOptions,
) -> Result<VizSvgDocument, VizError> {
    let projection = project_dataset(dataset, spec)?;
    render_projection_svg(&projection, spec, options)
}

/// Project graph-like input and render deterministic SVG.
pub fn render_graph_input_svg(
    input: &VizGraphInput,
    spec: &VizSpec,
    options: &VizRenderOptions,
) -> Result<VizSvgDocument, VizError> {
    let projection = project_graph_input(input, spec)?;
    render_projection_svg(&projection, spec, options)
}

/// Serialize a complete structured export as deterministic semantic SVG.
pub fn render_export_svg(export: &VizExport, options: &VizSvgOptions) -> Result<String, VizError> {
    let mut out = String::new();
    writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="100%" role="img" aria-labelledby="purrdf-title purrdf-desc" data-purrdf-schema="{}">"#,
        export.layout.width, export.layout.height, export.schema_version
    )
    .expect("writing to String cannot fail");
    write!(out, "<title id=\"purrdf-title\">").expect("writing to String cannot fail");
    escape_xml_text(&options.title, &mut out);
    out.push_str("</title>\n");
    writeln!(
        out,
        "<desc id=\"purrdf-desc\">{} mode RDF 1.2 visualization with {} terms, {} structural statements, {} assertions, and {} statement-layer relations.</desc>",
        mode_name(export.scene.mode),
        export.model.terms.len(),
        export.model.statements.len(),
        export.model.assertions.len(),
        export.model.relations.len()
    )
    .expect("writing to String cannot fail");
    if options.embed_metadata {
        let metadata = export_json(export)?;
        out.push_str(
            "<metadata id=\"purrdf-viz-export\" type=\"application/vnd.purrdf.viz+json\">",
        );
        escape_xml_text(&metadata, &mut out);
        out.push_str("</metadata>\n");
    }
    render_defs(export, &mut out, options.include_styles);
    writeln!(
        out,
        "<rect class=\"canvas\" x=\"0\" y=\"0\" width=\"{}\" height=\"{}\"/>",
        export.layout.width, export.layout.height
    )
    .expect("writing to String cannot fail");

    if export.scene.mode == VizMode::Table {
        render_table(export, &mut out);
    } else {
        render_graph(export, &mut out);
    }
    render_legend(export, &mut out);
    out.push_str("</svg>\n");
    Ok(out)
}

fn build_element_index(scene: &VizScene, layout: &VizLayout) -> Vec<VizElementIndexEntry> {
    let mut entries = Vec::new();
    for node in &scene.nodes {
        for (suffix, kind) in [
            ("group", VizElementKind::NodeGroup),
            ("shape", VizElementKind::NodeShape),
            ("label", VizElementKind::NodeLabel),
        ] {
            entries.push(index_entry(
                &format!("svg-{}-{suffix}", node.id),
                &node.id,
                node.bindings.clone(),
                kind,
            ));
        }
        for (index, badge) in node.badges.iter().enumerate() {
            let bindings = badge_bindings(badge, &node.bindings);
            entries.push(index_entry(
                &format!("svg-{}-badge-{index}", node.id),
                &node.id,
                bindings.clone(),
                VizElementKind::NodeBadge,
            ));
            entries.push(index_entry(
                &format!("svg-{}-badge-{index}-label", node.id),
                &node.id,
                bindings,
                VizElementKind::NodeBadgeLabel,
            ));
        }
        for port in &node.ports {
            entries.push(index_entry(
                &format!("svg-{}-port-{}", node.id, port.id),
                &node.id,
                node.bindings.clone(),
                VizElementKind::NodePort,
            ));
        }
    }
    for edge in &scene.edges {
        for (suffix, kind) in [
            ("group", VizElementKind::EdgeGroup),
            ("path", VizElementKind::EdgePath),
            ("label", VizElementKind::EdgeLabel),
        ] {
            entries.push(index_entry(
                &format!("svg-{}-{suffix}", edge.id),
                &edge.id,
                edge.bindings.clone(),
                kind,
            ));
        }
        for (index, badge) in edge.badges.iter().enumerate() {
            let bindings = badge_bindings(badge, &edge.bindings);
            entries.push(index_entry(
                &format!("svg-{}-badge-{index}", edge.id),
                &edge.id,
                bindings.clone(),
                VizElementKind::EdgeBadge,
            ));
            entries.push(index_entry(
                &format!("svg-{}-badge-{index}-label", edge.id),
                &edge.id,
                bindings,
                VizElementKind::EdgeBadgeLabel,
            ));
        }
        if let Some(anchor) = &edge.anchor {
            entries.push(index_entry(
                &format!("svg-{}-anchor", edge.id),
                &anchor.id,
                anchor.bindings.clone(),
                VizElementKind::EdgeAnchor,
            ));
            entries.push(index_entry(
                &format!("svg-{}-anchor-label", edge.id),
                &anchor.id,
                anchor.bindings.clone(),
                VizElementKind::EdgeAnchorLabel,
            ));
            for (index, badge) in anchor.badges.iter().enumerate() {
                let bindings = badge_bindings(badge, &anchor.bindings);
                entries.push(index_entry(
                    &format!("svg-{}-anchor-badge-{index}", edge.id),
                    &anchor.id,
                    bindings.clone(),
                    VizElementKind::EdgeAnchorBadge,
                ));
                entries.push(index_entry(
                    &format!("svg-{}-anchor-badge-{index}-label", edge.id),
                    &anchor.id,
                    bindings,
                    VizElementKind::EdgeAnchorBadgeLabel,
                ));
            }
        }
    }
    if let (Some(scene_table), Some(layout_table)) = (&scene.table, &layout.table) {
        entries.push(index_entry(
            "svg-table",
            "table",
            Vec::new(),
            VizElementKind::Table,
        ));
        for cell in &layout_table.cells {
            let (scene_id, bindings) = table_cell_binding(scene_table, cell.row, cell.column);
            entries.push(index_entry(
                &format!("svg-table-cell-{}-{}", cell.row, cell.column),
                &scene_id,
                bindings.clone(),
                VizElementKind::TableCell,
            ));
            entries.push(index_entry(
                &format!("svg-table-label-{}-{}", cell.row, cell.column),
                &scene_id,
                bindings,
                VizElementKind::TableLabel,
            ));
        }
    }
    entries.push(index_entry(
        "svg-legend",
        "group-legend",
        Vec::new(),
        VizElementKind::Legend,
    ));
    for entry in &scene.legend {
        entries.push(index_entry(
            &format!("svg-{}", entry.id),
            &entry.id,
            Vec::new(),
            VizElementKind::LegendEntry,
        ));
        entries.push(index_entry(
            &format!("svg-{}-label", entry.id),
            &entry.id,
            Vec::new(),
            VizElementKind::LegendLabel,
        ));
    }
    entries.sort_by(|left, right| left.element_id.cmp(&right.element_id));
    entries
}

fn table_cell_binding(
    table: &VizSceneTable,
    row: usize,
    column: usize,
) -> (String, Vec<VizSemanticRef>) {
    if row == 0 {
        return (format!("table-header-{column}"), Vec::new());
    }
    table.rows.get(row - 1).map_or_else(
        || (format!("table-row-{row}"), Vec::new()),
        |scene_row| {
            let bindings = scene_row
                .cells
                .get(column)
                .map_or_else(Vec::new, |cell| cell.bindings.clone());
            (scene_row.id.clone(), bindings)
        },
    )
}

fn index_entry(
    element_id: &str,
    scene_id: &str,
    bindings: Vec<VizSemanticRef>,
    kind: VizElementKind,
) -> VizElementIndexEntry {
    VizElementIndexEntry {
        element_id: element_id.to_owned(),
        scene_id: scene_id.to_owned(),
        bindings,
        kind,
    }
}

fn badge_bindings(badge: &VizBadge, owner: &[VizSemanticRef]) -> Vec<VizSemanticRef> {
    badge
        .binding
        .clone()
        .map_or_else(|| owner.to_vec(), |binding| vec![binding])
}

fn render_defs(export: &VizExport, out: &mut String, include_styles: bool) {
    out.push_str("<defs>\n");
    for (id, color) in [
        ("arrow-assertion", "#147d8a"),
        ("arrow-role", "#64748b"),
        ("arrow-reifies", "#b65c00"),
        ("arrow-annotation", "#257942"),
        ("arrow-quote", "#6f4ba8"),
    ] {
        writeln!(
            out,
            "<marker id=\"{id}\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"{color}\"/></marker>"
        )
        .expect("writing to String cannot fail");
    }
    for (id, rect) in text_clip_rects(export) {
        writeln!(
            out,
            "<clipPath id=\"clip-{id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>",
            rect.x, rect.y, rect.width, rect.height
        )
        .expect("writing to String cannot fail");
    }
    if include_styles {
        out.push_str("<style>");
        out.push_str(SVG_STYLE);
        out.push_str("</style>\n");
    }
    out.push_str("</defs>\n");
}

fn text_clip_rects(export: &VizExport) -> Vec<(String, VizRect)> {
    let mut clips = Vec::new();
    for node in &export.layout.nodes {
        clips.push((format!("svg-{}-label", node.id), node.label.rect));
        clips.extend(node.badges.iter().map(|badge| {
            (
                format!("svg-{}-badge-{}-label", node.id, badge.index),
                badge.rect,
            )
        }));
    }
    for edge in &export.layout.edges {
        clips.push((format!("svg-{}-label", edge.id), edge.label.rect));
        clips.extend(edge.badges.iter().map(|badge| {
            (
                format!("svg-{}-badge-{}-label", edge.id, badge.index),
                badge.rect,
            )
        }));
        if let Some(anchor) = &edge.anchor {
            clips.push((format!("svg-{}-anchor-label", edge.id), anchor.label.rect));
            clips.extend(anchor.badges.iter().map(|badge| {
                (
                    format!("svg-{}-anchor-badge-{}-label", edge.id, badge.index),
                    badge.rect,
                )
            }));
        }
    }
    if let Some(table) = &export.layout.table {
        clips.extend(table.cells.iter().map(|cell| {
            (
                format!("svg-table-label-{}-{}", cell.row, cell.column),
                cell.label.rect,
            )
        }));
    }
    clips.extend(export.layout.legend.iter().map(|entry| {
        (
            format!("svg-{}-label", entry.id),
            VizRect {
                x: entry.rect.x + 68,
                y: entry.rect.y + 6,
                width: entry.rect.width - 76,
                height: entry.rect.height - 12,
            },
        )
    }));
    clips.sort_by(|left, right| left.0.cmp(&right.0));
    clips
}

fn render_graph(export: &VizExport, out: &mut String) {
    let scene_edges = export
        .scene
        .edges
        .iter()
        .map(|edge| (edge.id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();
    for layout in &export.layout.edges {
        if let Some(scene) = scene_edges.get(layout.id.as_str()) {
            render_edge(scene, layout, out);
        }
    }
    let scene_nodes = export
        .scene
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    for layout in &export.layout.nodes {
        if let Some(scene) = scene_nodes.get(layout.id.as_str()) {
            render_node(scene, layout, out);
        }
    }
}

fn render_edge(scene: &VizSceneEdge, layout: &VizLayoutEdge, out: &mut String) {
    let class = edge_class(scene.kind);
    writeln!(
        out,
        "<g id=\"svg-{}-group\" class=\"edge {class}\" data-scene-id=\"{}\">",
        scene.id, scene.id
    )
    .expect("writing to String cannot fail");
    render_accessibility(&scene.accessibility, out);
    writeln!(
        out,
        "<path id=\"svg-{}-path\" class=\"edge-path\" d=\"{}\" marker-end=\"url(#{})\"/>",
        scene.id,
        path_data(&layout.points),
        edge_marker(scene.kind)
    )
    .expect("writing to String cannot fail");
    render_edge_label_leader(&layout.points, layout.label.rect, out);
    render_label_box(
        &format!("svg-{}-label", scene.id),
        &scene.label,
        &layout.label,
        "edge-label",
        out,
    );
    render_badges(&scene.id, &scene.badges, &layout.badges, "edge-badge", out);
    if let (Some(scene_anchor), Some(layout_anchor)) = (&scene.anchor, &layout.anchor) {
        render_anchor(&scene.id, scene_anchor, layout_anchor, out);
    }
    out.push_str("</g>\n");
}

fn render_edge_label_leader(points: &[VizPoint], label: VizRect, out: &mut String) {
    let center = VizPoint {
        x: label.x + label.width / 2,
        y: label.y + label.height / 2,
    };
    let Some(target) = nearest_point_on_path(points, center) else {
        return;
    };
    let inside_label = target.x >= label.x - 4
        && target.x <= label.x + label.width + 4
        && target.y >= label.y - 4
        && target.y <= label.y + label.height + 4;
    if inside_label {
        return;
    }
    writeln!(
        out,
        "<path class=\"edge-label-leader\" d=\"M {} {} L {} {}\"/>",
        center.x, center.y, target.x, target.y
    )
    .expect("writing to String cannot fail");
}

fn render_anchor(edge_id: &str, scene: &VizEdgeAnchor, layout: &VizLayoutAnchor, out: &mut String) {
    writeln!(
        out,
        "<g id=\"svg-{edge_id}-anchor\" class=\"statement-anchor\" data-scene-id=\"{}\">",
        scene.id
    )
    .expect("writing to String cannot fail");
    render_accessibility(&scene.accessibility, out);
    render_rect(layout.rect, "anchor-shape", out);
    render_text(
        &format!("svg-{edge_id}-anchor-label"),
        &scene.label,
        &layout.label,
        "anchor-label",
        out,
    );
    render_badges(
        &format!("{edge_id}-anchor"),
        &scene.badges,
        &layout.badges,
        "anchor-badge",
        out,
    );
    out.push_str("</g>\n");
}

fn render_node(scene: &VizSceneNode, layout: &VizLayoutNode, out: &mut String) {
    let mut class = format!("node {}", node_class(scene.kind));
    if scene
        .badges
        .iter()
        .any(|badge| badge.kind == VizBadgeKind::Focus)
    {
        class.push_str(" focus");
    }
    if scene
        .badges
        .iter()
        .any(|badge| badge.kind == VizBadgeKind::Reifier)
    {
        class.push_str(" reifier");
    }
    writeln!(
        out,
        "<g id=\"svg-{}-group\" class=\"{class}\" data-scene-id=\"{}\">",
        scene.id, scene.id
    )
    .expect("writing to String cannot fail");
    render_accessibility(&scene.accessibility, out);
    write!(out, "<rect id=\"svg-{}-shape\" ", scene.id).expect("writing to String cannot fail");
    render_rect_attributes(layout.rect, "node-shape", out);
    out.push_str("/>\n");
    if scene.kind == VizSceneNodeKind::Statement {
        let inner = VizRect {
            x: layout.rect.x + 4,
            y: layout.rect.y + 4,
            width: layout.rect.width - 8,
            height: layout.rect.height - 8,
        };
        render_rect(inner, "statement-inner", out);
    }
    render_text(
        &format!("svg-{}-label", scene.id),
        &scene.label,
        &layout.label,
        "node-label",
        out,
    );
    render_badges(&scene.id, &scene.badges, &layout.badges, "node-badge", out);
    for port in &layout.ports {
        writeln!(
            out,
            "<circle id=\"svg-{}-port-{}\" class=\"node-port\" cx=\"{}\" cy=\"{}\" r=\"3\"/>",
            scene.id, port.id, port.point.x, port.point.y
        )
        .expect("writing to String cannot fail");
    }
    out.push_str("</g>\n");
}

fn render_table(export: &VizExport, out: &mut String) {
    let (Some(scene), Some(layout)) = (&export.scene.table, &export.layout.table) else {
        return;
    };
    out.push_str("<g id=\"svg-table\" class=\"statement-table\">\n");
    for cell in &layout.cells {
        let text = if cell.row == 0 {
            scene
                .fields
                .get(cell.column)
                .map_or("", |field| table_field_label(*field))
        } else {
            scene
                .rows
                .get(cell.row - 1)
                .and_then(|row| row.cells.get(cell.column))
                .map_or("", |value| value.text.as_str())
        };
        let class = if cell.row == 0 {
            "table-cell header"
        } else if cell.row % 2 == 0 {
            "table-cell even"
        } else {
            "table-cell odd"
        };
        writeln!(
            out,
            "<g id=\"svg-table-cell-{}-{}\" class=\"{class}\">",
            cell.row, cell.column
        )
        .expect("writing to String cannot fail");
        render_rect(cell.rect, "table-cell-shape", out);
        render_text(
            &format!("svg-table-label-{}-{}", cell.row, cell.column),
            &plain_scene_label(text),
            &cell.label,
            "table-label",
            out,
        );
        out.push_str("</g>\n");
    }
    out.push_str("</g>\n");
}

fn render_legend(export: &VizExport, out: &mut String) {
    let layout = export
        .layout
        .legend
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    out.push_str("<g id=\"svg-legend\" class=\"legend\">\n");
    for entry in &export.scene.legend {
        let Some(geometry) = layout.get(entry.id.as_str()) else {
            continue;
        };
        writeln!(
            out,
            "<g id=\"svg-{}\" class=\"legend-entry {}\">",
            entry.id, entry.id
        )
        .expect("writing to String cannot fail");
        render_rect(geometry.rect, "legend-entry-shape", out);
        render_legend_symbol(entry, geometry.rect, out);
        let label_rect = VizLayoutLabel {
            rect: VizRect {
                x: geometry.rect.x + 68,
                y: geometry.rect.y + 6,
                width: geometry.rect.width - 76,
                height: geometry.rect.height - 12,
            },
            lines: wrap_text(&entry.label, chars_for_width(geometry.rect.width - 76)),
        };
        render_text(
            &format!("svg-{}-label", entry.id),
            &plain_scene_label(&entry.label),
            &label_rect,
            "legend-label",
            out,
        );
        out.push_str("</g>\n");
    }
    out.push_str("</g>\n");
}

fn render_legend_symbol(entry: &VizLegendEntry, rect: VizRect, out: &mut String) {
    let x1 = rect.x + 12;
    let x2 = rect.x + 56;
    let y = rect.y + rect.height / 2;
    let class = if entry.id.contains("reifies") {
        "reifies"
    } else if entry.id.contains("annotation") {
        "annotation"
    } else if entry.id.contains("quoted") || entry.id.contains("statement") {
        "quoted"
    } else if entry.id.contains("subject")
        || entry.id.contains("predicate")
        || entry.id.contains("object")
    {
        "role"
    } else {
        "assertion"
    };
    if entry.id.contains("anchor") || entry.id.contains("quoted") || entry.id.contains("statement")
    {
        writeln!(
            out,
            "<rect class=\"legend-symbol {class}\" x=\"{}\" y=\"{}\" width=\"44\" height=\"22\" rx=\"4\"/>",
            x1,
            y - 11
        )
        .expect("writing to String cannot fail");
    } else {
        writeln!(
            out,
            "<path class=\"legend-line {class}\" d=\"M {x1} {y} H {x2}\" marker-end=\"url(#{})\"/>",
            match class {
                "reifies" => "arrow-reifies",
                "annotation" => "arrow-annotation",
                "role" => "arrow-role",
                _ => "arrow-assertion",
            }
        )
        .expect("writing to String cannot fail");
    }
}

fn render_label_box(
    id: &str,
    scene: &VizSceneLabel,
    layout: &VizLayoutLabel,
    class: &str,
    out: &mut String,
) {
    render_rect(layout.rect, &format!("{class}-box"), out);
    render_text(id, scene, layout, class, out);
}

fn render_text(
    id: &str,
    scene: &VizSceneLabel,
    layout: &VizLayoutLabel,
    class: &str,
    out: &mut String,
) {
    write!(
        out,
        "<text id=\"{id}\" class=\"{class}\" text-anchor=\"middle\" unicode-bidi=\"isolate\" clip-path=\"url(#clip-{id})\""
    )
    .expect("writing to String cannot fail");
    if let Some(language) = &scene.language {
        out.push_str(" lang=\"");
        escape_xml_attr(language, out);
        out.push('"');
    }
    if let Some(direction) = scene.direction {
        write!(
            out,
            " direction=\"{}\"",
            match direction {
                VizTextDirection::Ltr => "ltr",
                VizTextDirection::Rtl => "rtl",
            }
        )
        .expect("writing to String cannot fail");
    }
    out.push('>');
    out.push_str("<title>");
    escape_xml_text(&scene.full_text, out);
    out.push_str("</title>");
    let line_count = i32::try_from(layout.lines.len()).unwrap_or(1);
    let first_y = layout.rect.y + layout.rect.height / 2 - (line_count - 1) * 9 + 5;
    for (index, line) in layout.lines.iter().enumerate() {
        write!(
            out,
            "<tspan x=\"{}\" y=\"{}\">",
            layout.rect.x + layout.rect.width / 2,
            first_y + i32::try_from(index).unwrap_or_default() * 18
        )
        .expect("writing to String cannot fail");
        escape_xml_text(line, out);
        out.push_str("</tspan>");
    }
    out.push_str("</text>\n");
}

fn render_badges(
    owner_id: &str,
    scene: &[VizBadge],
    layout: &[VizLayoutBadge],
    class: &str,
    out: &mut String,
) {
    for geometry in layout {
        let Some(badge) = scene.get(geometry.index) else {
            continue;
        };
        let kind = badge_class(badge.kind);
        writeln!(
            out,
            "<g id=\"svg-{owner_id}-badge-{}\" class=\"{class} {kind}\">",
            geometry.index
        )
        .expect("writing to String cannot fail");
        render_rect(geometry.rect, "badge-shape", out);
        let label = VizLayoutLabel {
            rect: geometry.rect,
            lines: vec![badge.label.clone()],
        };
        render_text(
            &format!("svg-{owner_id}-badge-{}-label", geometry.index),
            &plain_scene_label(&badge.label),
            &label,
            "badge-label",
            out,
        );
        out.push_str("</g>\n");
    }
}

fn render_accessibility(value: &VizAccessibility, out: &mut String) {
    out.push_str("<title>");
    escape_xml_text(&value.title, out);
    out.push_str("</title><desc>");
    escape_xml_text(&value.description, out);
    out.push_str("</desc>\n");
}

fn render_rect(rect: VizRect, class: &str, out: &mut String) {
    write!(out, "<rect ").expect("writing to String cannot fail");
    render_rect_attributes(rect, class, out);
    out.push_str("/>\n");
}

fn render_rect_attributes(rect: VizRect, class: &str, out: &mut String) {
    let radius = match class {
        "table-cell-shape" | "text-clip" => 0,
        "badge-shape" | "edge-label-box" | "legend-entry-shape" => 4,
        _ => 6,
    };
    write!(
        out,
        "class=\"{class}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\"",
        rect.x, rect.y, rect.width, rect.height,
    )
    .expect("writing to String cannot fail");
}

fn path_data(points: &[VizPoint]) -> String {
    let mut out = String::new();
    if let Some(first) = points.first() {
        write!(out, "M {} {}", first.x, first.y).expect("writing to String cannot fail");
        let mut previous = *first;
        for point in points.iter().skip(1) {
            if point.x == previous.x {
                write!(out, " V {}", point.y).expect("writing to String cannot fail");
            } else if point.y == previous.y {
                write!(out, " H {}", point.x).expect("writing to String cannot fail");
            } else {
                write!(out, " L {} {}", point.x, point.y).expect("writing to String cannot fail");
            }
            previous = *point;
        }
    }
    out
}

fn nearest_point_on_path(points: &[VizPoint], point: VizPoint) -> Option<VizPoint> {
    points
        .windows(2)
        .map(|pair| {
            let start = pair[0];
            let end = pair[1];
            let candidate = if start.x == end.x {
                VizPoint {
                    x: start.x,
                    y: point.y.clamp(start.y.min(end.y), start.y.max(end.y)),
                }
            } else if start.y == end.y {
                VizPoint {
                    x: point.x.clamp(start.x.min(end.x), start.x.max(end.x)),
                    y: start.y,
                }
            } else {
                start
            };
            let dx = i64::from(candidate.x - point.x);
            let dy = i64::from(candidate.y - point.y);
            (dx * dx + dy * dy, candidate)
        })
        .min_by_key(|(distance, candidate)| (*distance, candidate.x, candidate.y))
        .map(|(_, candidate)| candidate)
}

fn edge_class(kind: VizSceneEdgeKind) -> &'static str {
    match kind {
        VizSceneEdgeKind::Assertion => "assertion",
        VizSceneEdgeKind::Subject | VizSceneEdgeKind::Predicate | VizSceneEdgeKind::Object => {
            "role"
        }
        VizSceneEdgeKind::Reifies => "reifies",
        VizSceneEdgeKind::Annotation => "annotation",
        VizSceneEdgeKind::QuoteSubject | VizSceneEdgeKind::QuoteObject => "quoted",
    }
}

fn edge_marker(kind: VizSceneEdgeKind) -> &'static str {
    match kind {
        VizSceneEdgeKind::Assertion => "arrow-assertion",
        VizSceneEdgeKind::Subject | VizSceneEdgeKind::Predicate | VizSceneEdgeKind::Object => {
            "arrow-role"
        }
        VizSceneEdgeKind::Reifies => "arrow-reifies",
        VizSceneEdgeKind::Annotation => "arrow-annotation",
        VizSceneEdgeKind::QuoteSubject | VizSceneEdgeKind::QuoteObject => "arrow-quote",
    }
}

fn node_class(kind: VizSceneNodeKind) -> &'static str {
    match kind {
        VizSceneNodeKind::Iri => "iri",
        VizSceneNodeKind::Blank => "blank",
        VizSceneNodeKind::Literal => "literal",
        VizSceneNodeKind::Statement => "statement",
    }
}

fn badge_class(kind: VizBadgeKind) -> &'static str {
    match kind {
        VizBadgeKind::Asserted => "asserted",
        VizBadgeKind::Quoted => "quoted",
        VizBadgeKind::Reifier => "reifier",
        VizBadgeKind::Graph => "graph",
        VizBadgeKind::AnnotationCount => "annotation-count",
        VizBadgeKind::ReifierCount => "reifier-count",
        VizBadgeKind::ReferenceCount => "reference-count",
        VizBadgeKind::NestingDepth => "nesting-depth",
        VizBadgeKind::Dialect => "dialect",
        VizBadgeKind::Direction => "direction",
        VizBadgeKind::Focus => "focus",
        VizBadgeKind::Role => "role",
    }
}

fn mode_name(mode: VizMode) -> &'static str {
    match mode {
        VizMode::Compact => "Compact resource graph",
        VizMode::Incidence => "Exact statement incidence graph",
        VizMode::Table => "Statement table",
    }
}

fn table_field_label(field: VizTableField) -> &'static str {
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

fn plain_scene_label(value: &str) -> VizSceneLabel {
    VizSceneLabel {
        text: value.to_owned(),
        full_text: value.to_owned(),
        language: None,
        direction: None,
    }
}

fn escape_xml_attr(value: &str, out: &mut String) {
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(character),
        }
    }
}

fn escape_xml_text(value: &str, out: &mut String) {
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(character),
        }
    }
}

const SVG_STYLE: &str = r"
.canvas{fill:#f6f8fb}
text{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;letter-spacing:0;fill:#17202a}
.node-shape{stroke-width:2;fill:#fff;stroke:#176b78}
.node.blank .node-shape{stroke:#64748b;stroke-dasharray:7 4;fill:#f8fafc}
.node.literal .node-shape{stroke:#8a5a00;fill:#fff7e6}
.node.statement .node-shape{stroke:#6f4ba8;stroke-width:2.5;fill:#f7f3ff;stroke-dasharray:8 4}
.statement-inner{fill:none;stroke:#b6a1d2;stroke-width:1}
.node.reifier .node-shape{stroke:#b65c00;fill:#fff8ed;stroke-width:3}
.node.focus .node-shape{stroke:#a53a3a;stroke-width:4}
.node-label{font-size:14px;font-weight:700}
.node-port{fill:#fff;stroke:#334155;stroke-width:1.5}
.edge-path,.legend-line{fill:none;stroke-width:2.5;stroke-linejoin:round;stroke-linecap:round}
.edge-label-leader{fill:none;stroke:#8392a5;stroke-width:1;stroke-dasharray:2 3}
.edge.assertion .edge-path,.legend-line.assertion{stroke:#147d8a}
.edge.role .edge-path,.legend-line.role{stroke:#64748b;stroke-width:1.8}
.edge.reifies .edge-path,.legend-line.reifies{stroke:#b65c00;stroke-width:2.8;stroke-dasharray:10 4}
.edge.annotation .edge-path,.legend-line.annotation{stroke:#257942;stroke-width:2.3}
.edge.quoted .edge-path{stroke:#6f4ba8;stroke-width:1.7;stroke-dasharray:3 5}
.edge-label-box{fill:#fff;stroke:#aeb9c7;stroke-width:1}
.edge.assertion .edge-label-box{fill:#eef9fa;stroke:#66a8b0}
.edge.role .edge-label-box{fill:#f7f8fa;stroke:#aeb7c2}
.edge.reifies .edge-label-box{fill:#fff6e9;stroke:#d18a3d}
.edge.annotation .edge-label-box{fill:#edf8f1;stroke:#69a67c}
.edge.quoted .edge-label-box{fill:#f7f2fd;stroke:#9d82c1}
.edge-label{font-size:12px;font-weight:650}
.statement-anchor .anchor-shape{fill:#e5f4f5;stroke:#147d8a;stroke-width:2.5}
.anchor-label{fill:#173f48;font-size:12px;font-weight:700}
.badge-shape{fill:#eef2f6;stroke:#94a3b8;stroke-width:1}
.graph .badge-shape{fill:#e7f5ef;stroke:#277a5a}
.quoted .badge-shape{fill:#f1ebfa;stroke:#6f4ba8}
.reifier .badge-shape,.reifier-count .badge-shape{fill:#fff0dc;stroke:#b65c00}
.dialect .badge-shape,.focus .badge-shape{fill:#fdecec;stroke:#a53a3a}
.direction .badge-shape{fill:#e8f0ff;stroke:#315d9b}
.badge-label{font-size:10px;font-weight:650}
.table-cell-shape{stroke:#c4ceda;stroke-width:1;fill:#fff}
.table-cell.header .table-cell-shape{fill:#173f48;stroke:#173f48}
.table-cell.header .table-label{fill:#fff;font-weight:700}
.table-cell.even .table-cell-shape{fill:#f0f4f8}
.table-label{font-size:12px}
.legend-entry-shape{fill:#fff;stroke:#c4ceda;stroke-width:1}
.legend-label{font-size:11px}
.legend-symbol.assertion{fill:#e5f4f5;stroke:#147d8a;stroke-width:2}
.legend-symbol.quoted{fill:#f7f3ff;stroke:#6f4ba8;stroke-width:2;stroke-dasharray:5 3}
";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const EX: &str = "https://example.org/";

    fn iri(local: &str) -> TermValue {
        TermValue::Iri(format!("{EX}{local}"))
    }

    fn input(asserted: bool) -> VizGraphInput {
        VizGraphInput {
            quads: asserted
                .then(|| VizInputQuad {
                    subject: iri("alice"),
                    predicate: format!("{EX}knows"),
                    object: iri("bob"),
                    graph_name: Some(iri("facts")),
                })
                .into_iter()
                .collect(),
            reifiers: vec![VizInputReifier {
                reifier: iri("claim<&>"),
                statement: VizInputStatement {
                    subject: iri("alice"),
                    predicate: format!("{EX}knows"),
                    object: iri("bob"),
                },
                graph_name: Some(iri("claims")),
            }],
            annotations: vec![VizInputAnnotation {
                reifier: iri("claim<&>"),
                predicate: format!("{EX}message"),
                object: TermValue::Literal {
                    lexical_form: "مرحبا <world> & everyone".to_owned(),
                    datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".to_owned(),
                    language: Some("ar".to_owned()),
                    direction: Some(RdfTextDirection::Rtl),
                },
                graph_name: Some(iri("provenance")),
            }],
        }
    }

    #[test]
    fn svg_round_trips_embedded_export_and_element_ids() {
        let document = render_graph_input_svg(
            &input(true),
            &VizSpec::default(),
            &VizRenderOptions::default(),
        )
        .expect("svg");
        let xml = roxmltree::Document::parse(&document.svg).expect("valid XML");
        let metadata = xml
            .descendants()
            .find(|node| node.has_tag_name("metadata"))
            .and_then(|node| node.text())
            .expect("metadata text");
        let decoded: VizExport = serde_json::from_str(metadata).expect("export JSON");
        assert_eq!(decoded, document.export);
        let all_ids = xml
            .descendants()
            .filter_map(|node| node.attribute("id"))
            .collect::<Vec<_>>();
        assert_eq!(
            all_ids.len(),
            all_ids.iter().copied().collect::<BTreeSet<_>>().len(),
            "SVG ids must be unique"
        );
        let indexed = document
            .export
            .element_index
            .iter()
            .map(|entry| entry.element_id.as_str())
            .collect::<BTreeSet<_>>();
        for id in all_ids.iter().filter(|id| id.starts_with("svg-")) {
            assert!(indexed.contains(id), "semantic SVG id {id} is not indexed");
        }
        for entry in &document.export.element_index {
            assert!(
                xml.descendants()
                    .any(|node| node.attribute("id") == Some(entry.element_id.as_str()))
            );
        }
        let clip_paths = xml
            .descendants()
            .filter(|node| node.has_tag_name("clipPath"))
            .collect::<Vec<_>>();
        let text_count = xml
            .descendants()
            .filter(|node| node.has_tag_name("text"))
            .count();
        assert_eq!(clip_paths.len(), text_count);
        assert!(clip_paths.iter().all(|clip_path| {
            clip_path
                .children()
                .find(|node| node.has_tag_name("rect"))
                .is_some_and(|rect| {
                    rect.attribute("width")
                        .and_then(|value| value.parse::<i32>().ok())
                        .is_some_and(|value| value > 0)
                        && rect
                            .attribute("height")
                            .and_then(|value| value.parse::<i32>().ok())
                            .is_some_and(|value| value > 0)
                })
        }));
    }

    #[test]
    fn svg_escapes_rdf_text_and_preserves_bidi_attributes() {
        let document = render_graph_input_svg(
            &input(true),
            &VizSpec::default(),
            &VizRenderOptions::default(),
        )
        .expect("svg");
        assert!(document.svg.contains("&lt;world&gt; &amp; everyone"));
        assert!(document.svg.contains("direction=\"rtl\""));
        assert!(document.svg.contains("lang=\"ar\""));
        assert!(!document.svg.contains("مرحبا <world>"));
    }

    #[test]
    fn asserted_and_quoted_only_svgs_have_distinct_grammar() {
        let asserted = render_graph_input_svg(
            &input(true),
            &VizSpec::default(),
            &VizRenderOptions::default(),
        )
        .expect("asserted");
        let quoted = render_graph_input_svg(
            &input(false),
            &VizSpec::default(),
            &VizRenderOptions::default(),
        )
        .expect("quoted");
        assert!(asserted.svg.contains("class=\"edge assertion\""));
        assert!(asserted.svg.contains("class=\"statement-anchor\""));
        assert!(!quoted.svg.contains("class=\"edge assertion\""));
        assert!(quoted.svg.contains("class=\"node statement\""));
        assert_ne!(asserted.svg, quoted.svg);
    }

    #[test]
    fn table_svg_is_a_real_table_without_graph_edges() {
        let spec = VizSpec {
            mode: VizMode::Table,
            table_fields: vec![VizTableField::Statement, VizTableField::Reifiers],
            ..VizSpec::default()
        };
        let document = render_graph_input_svg(&input(true), &spec, &VizRenderOptions::default())
            .expect("table");
        assert!(document.svg.contains("class=\"statement-table\""));
        assert!(!document.svg.contains("class=\"edge-path\""));
        assert!(document.export.scene.nodes.is_empty());
        assert!(document.export.layout.table.is_some());
    }

    #[test]
    fn svg_bytes_are_deterministic() {
        let first = render_graph_input_svg(
            &input(true),
            &VizSpec::default(),
            &VizRenderOptions::default(),
        )
        .expect("first");
        let second = render_graph_input_svg(
            &input(true),
            &VizSpec::default(),
            &VizRenderOptions::default(),
        )
        .expect("second");
        assert_eq!(first, second);
        assert_eq!(first.export.schema_version, VIZ_EXPORT_SCHEMA_VERSION);
        assert!(!first.export.spec_hash.is_empty());
        assert!(!first.export.model_hash.is_empty());
        assert!(!first.export.scene_hash.is_empty());
    }
}
