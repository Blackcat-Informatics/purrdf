// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Enriched export assembly and emission through the native SVG primitives.

use super::{
    VizSvgOptions, assemble_export, escape_xml_text, index_entry, path_data, render_document,
    render_label_box, render_rect, render_rect_attributes, render_text, xml_attribute,
};
use crate::viz::layout::layout_presented;
use crate::viz::presentation::presentation_error;
use crate::viz::{
    VIZ_PRESENTED_SCHEMA_VERSION, VizElementIndexEntry, VizElementKind, VizError, VizExport,
    VizInteractionLayout, VizLayoutOptions, VizNavigation, VizPresentation, VizPresentationIssue,
    VizPresentationLayout, VizPresentedExport, VizProjection, VizRegionKind, VizSceneLabel,
    VizSemanticRef, VizSpec, build_presented_scene, stable_hash_hex,
};
use purrdf_lex::json::record::ToJson;
use std::fmt::Write as _;

/// Build a complete enriched export with authoritative geometry, hashes and index.
///
/// Scenario instructions supplement a visibly rendered RDF detail scene. They
/// never replace assertion/quotation, reifier, graph or directional-literal grammar.
pub fn build_presented_export(
    projection: &VizProjection,
    spec: &VizSpec,
    presentation: &VizPresentation,
    layout_options: &VizLayoutOptions,
) -> Result<VizPresentedExport, VizError> {
    let scene = build_presented_scene(projection, spec, presentation)?;
    let (layout, geometry) = layout_presented(projection, &scene, layout_options)?;
    let presentation_json = purrdf_lex::json::write_compact(&scene.presentation.to_json());
    let scene_json = purrdf_lex::json::write_compact(&scene.to_json());
    let export = assemble_export(projection, spec, scene.scene, layout);
    let element_index = extend_index(&export, &scene.presentation, &geometry)?;
    Ok(VizPresentedExport {
        schema_version: VIZ_PRESENTED_SCHEMA_VERSION.to_owned(),
        presentation: scene.presentation,
        presentation_hash: stable_hash_hex(&presentation_json),
        presented_scene_hash: stable_hash_hex(&scene_json),
        layout_options: layout_options.clone(),
        export,
        geometry,
        element_index,
    })
}

/// Serialize a provider-built enriched export through the native JSON record codec.
pub fn presented_export_json(export: &VizPresentedExport) -> Result<String, VizError> {
    validate_export(export)?;
    Ok(purrdf_lex::json::write_compact(&export.to_json()))
}

/// Render the exact geometry and semantic bindings in a verified enriched export.
///
/// Public record construction and JSON decoding cannot bypass semantic admission:
/// a changed scene, index, hash, instruction or layout must be rebuilt by the factory.
pub fn render_presented_export_svg(
    export: &VizPresentedExport,
    options: &VizSvgOptions,
) -> Result<String, VizError> {
    validate_export(export)?;
    render_document(&export.export, options, Some(export))
}

fn validate_export(export: &VizPresentedExport) -> Result<(), VizError> {
    let expected = build_presented_export(
        &export.export.model,
        &export.export.spec,
        &export.presentation,
        &export.layout_options,
    )?;
    if expected != *export {
        return Err(presentation_error(
            VizPresentationIssue::Export,
            "export",
            "rebuild the complete enriched export after changing its model, presentation or geometry",
        ));
    }
    Ok(())
}

fn navigation<'a>(
    bindings: &[VizSemanticRef],
    p: Option<&'a VizPresentation>,
) -> Result<Option<&'a VizNavigation>, VizError> {
    let mut links = p
        .into_iter()
        .flat_map(|p| &p.displays)
        .filter(|display| bindings.contains(&display.binding))
        .filter_map(|display| display.navigation.as_ref());
    let link = links.next();
    if links.any(|other| Some(other) != link) {
        return Err(presentation_error(
            VizPresentationIssue::Navigation,
            "semantic element",
            "assign at most one navigation destination to identities sharing a visible element",
        ));
    }
    Ok(link)
}

pub(super) fn open_navigation(
    id: &str,
    bindings: &[VizSemanticRef],
    p: Option<&VizPresentation>,
    out: &mut String,
) -> Result<bool, VizError> {
    let Some(link) = navigation(bindings, p)? else {
        return Ok(false);
    };
    writeln!(
        out,
        "<a id=\"svg-{}-link\" href=\"{}\">",
        xml_attribute(id)?,
        xml_attribute(link.as_str())?
    )
    .expect("writing to String cannot fail");
    Ok(true)
}

fn index_navigation(
    entries: &mut Vec<VizElementIndexEntry>,
    id: &str,
    bindings: &[VizSemanticRef],
    p: &VizPresentation,
) -> Result<(), VizError> {
    if navigation(bindings, Some(p))?.is_some() {
        entries.push(index_entry(
            &format!("svg-{id}-link"),
            id,
            bindings.to_vec(),
            VizElementKind::Navigation,
        ));
    }
    Ok(())
}

fn extend_index(
    export: &VizExport,
    p: &VizPresentation,
    geometry: &VizPresentationLayout,
) -> Result<Vec<VizElementIndexEntry>, VizError> {
    let mut entries = export.element_index.clone();
    for node in &export.scene.nodes {
        index_navigation(&mut entries, &node.id, &node.bindings, p)?;
    }
    for edge in &export.scene.edges {
        index_navigation(&mut entries, &edge.id, &edge.bindings, p)?;
    }
    for frame in geometry.components.iter().chain(&geometry.regions) {
        entries.push(index_entry(
            &format!("svg-{}-frame", frame.id),
            &frame.id,
            frame.bindings.clone(),
            VizElementKind::PresentationFrame,
        ));
        entries.push(index_entry(
            &format!("svg-{}-label", frame.id),
            &frame.id,
            frame.bindings.clone(),
            VizElementKind::PresentationFrameLabel,
        ));
        index_navigation(&mut entries, &frame.id, &frame.bindings, p)?;
    }
    for port in &geometry.ports {
        entries.push(index_entry(
            &format!("svg-{}", port.id),
            &port.boundary,
            port.bindings.clone(),
            VizElementKind::BoundaryPort,
        ));
    }
    for participant in &geometry.participants {
        let bindings = vec![VizSemanticRef::Term(participant.term.clone())];
        for (suffix, kind) in [
            ("group", VizElementKind::NodeGroup),
            ("shape", VizElementKind::NodeShape),
            ("label", VizElementKind::NodeLabel),
            ("lifeline", VizElementKind::Lifeline),
        ] {
            entries.push(index_entry(
                &format!("svg-{}-{suffix}", participant.id),
                &participant.id,
                bindings.clone(),
                kind,
            ));
        }
        index_navigation(&mut entries, &participant.id, &bindings, p)?;
    }
    for interaction in &geometry.interactions {
        let bindings = vec![interaction.binding.clone()];
        for (suffix, kind) in [
            ("group", VizElementKind::EdgeGroup),
            ("path", VizElementKind::EdgePath),
            ("label", VizElementKind::EdgeLabel),
        ] {
            entries.push(index_entry(
                &format!("svg-{}-{suffix}", interaction.id),
                &interaction.id,
                bindings.clone(),
                kind,
            ));
        }
        index_navigation(&mut entries, &interaction.id, &bindings, p)?;
    }
    if geometry.detail_heading.is_some() {
        entries.push(index_entry(
            "svg-present-detail-heading",
            "present-detail",
            Vec::new(),
            VizElementKind::DetailHeading,
        ));
    }
    entries.sort_by(|a, b| a.element_id.cmp(&b.element_id));
    if entries
        .windows(2)
        .any(|pair| pair[0].element_id == pair[1].element_id)
    {
        return Err(presentation_error(
            VizPresentationIssue::Export,
            "element index",
            "use presentation identifiers that do not collide with provider-owned elements",
        ));
    }
    Ok(entries)
}

pub(super) fn render_clips(g: &VizPresentationLayout, out: &mut String) -> Result<(), VizError> {
    out.push_str("<defs>\n");
    for (id, label) in g
        .components
        .iter()
        .chain(&g.regions)
        .map(|f| (format!("svg-{}-label", f.id), &f.text))
        .chain(
            g.participants
                .iter()
                .map(|p| (format!("svg-{}-label", p.id), &p.text)),
        )
        .chain(
            g.interactions
                .iter()
                .map(|i| (format!("svg-{}-label", i.id), &i.text)),
        )
        .chain(
            g.detail_heading
                .iter()
                .map(|label| ("svg-present-detail-heading".to_owned(), label)),
        )
    {
        write!(out, "<clipPath id=\"clip-{}\">", xml_attribute(&id)?)
            .expect("writing to String cannot fail");
        render_rect(label.rect, "presentation-clip", out);
        out.push_str("</clipPath>\n");
    }
    out.push_str("</defs>\n");
    Ok(())
}

fn frame_style(kind: Option<VizRegionKind>) -> &'static str {
    match kind {
        Some(VizRegionKind::Error) => {
            "fill:#fff7ed;fill-opacity:0.35;stroke:#7c2d12;stroke-width:3;stroke-dasharray:8 3"
        }
        Some(VizRegionKind::Parallel | VizRegionKind::Lane) => {
            "fill:#f8fafc;fill-opacity:0.25;stroke:#334155;stroke-width:2;stroke-dasharray:3 4"
        }
        Some(_) => "fill:#f8fafc;fill-opacity:0.25;stroke:#334155;stroke-width:2",
        None => "fill:#f1f5f9;fill-opacity:0.35;stroke:#334155;stroke-width:2",
    }
}

pub(super) fn render_geometry(p: &VizPresentedExport, out: &mut String) -> Result<(), VizError> {
    let g = &p.geometry;
    for frame in g.components.iter().chain(&g.regions) {
        let linked = open_navigation(&frame.id, &frame.bindings, Some(&p.presentation), out)?;
        writeln!(
            out,
            "<g id=\"svg-{}-frame\" role=\"group\" aria-label=\"{}\">",
            xml_attribute(&frame.id)?,
            xml_attribute(&frame.label.full_text)?
        )
        .expect("writing to String cannot fail");
        out.push_str("<title>");
        escape_xml_text(&frame.label.full_text, out)?;
        out.push_str("</title>\n");
        write!(out, "<rect style=\"{}\" ", frame_style(frame.kind))
            .expect("writing to String cannot fail");
        render_rect_attributes(frame.rect, "presentation-frame", out);
        out.push_str("/>\n");
        render_text(
            &format!("svg-{}-label", frame.id),
            &frame.label,
            &frame.text,
            "presentation-label",
            out,
        )?;
        out.push_str("</g>\n");
        if linked {
            out.push_str("</a>\n");
        }
    }
    for participant in &g.participants {
        let bindings = [VizSemanticRef::Term(participant.term.clone())];
        let linked = open_navigation(&participant.id, &bindings, Some(&p.presentation), out)?;
        writeln!(
            out,
            "<g id=\"svg-{}-group\" role=\"group\" aria-label=\"{}\"><title>{}</title>",
            xml_attribute(&participant.id)?,
            xml_attribute(&participant.label.full_text)?,
            xml_attribute(&participant.label.full_text)?
        )
        .expect("writing to String cannot fail");
        write!(
            out,
            "<rect id=\"svg-{}-shape\" style=\"fill:white;stroke:#334155;stroke-width:2\" ",
            xml_attribute(&participant.id)?
        )
        .expect("writing to String cannot fail");
        render_rect_attributes(participant.rect, "participant", out);
        out.push_str("/>\n");
        render_text(
            &format!("svg-{}-label", participant.id),
            &participant.label,
            &participant.text,
            "participant-label",
            out,
        )?;
        writeln!(out, "<path id=\"svg-{}-lifeline\" d=\"M {} {} L {} {}\" style=\"fill:none;stroke:#475569;stroke-dasharray:6 4\"/>", xml_attribute(&participant.id)?, participant.start.x, participant.start.y, participant.end.x, participant.end.y).expect("writing to String cannot fail");
        out.push_str("</g>\n");
        if linked {
            out.push_str("</a>\n");
        }
    }
    for interaction in &g.interactions {
        render_interaction(interaction, &p.presentation, out)?;
    }
    for port in &g.ports {
        writeln!(out, "<circle id=\"svg-{}\" cx=\"{}\" cy=\"{}\" r=\"4\" style=\"fill:white;stroke:#334155;stroke-width:2\"/>", xml_attribute(&port.id)?, port.point.x, port.point.y).expect("writing to String cannot fail");
    }
    if let Some(heading) = &g.detail_heading {
        render_text(
            "svg-present-detail-heading",
            &VizSceneLabel::plain("RDF detail — assertions, quotations and graph context"),
            heading,
            "detail-heading",
            out,
        )?;
    }
    Ok(())
}

fn render_interaction(
    i: &VizInteractionLayout,
    p: &VizPresentation,
    out: &mut String,
) -> Result<(), VizError> {
    let linked = open_navigation(&i.id, std::slice::from_ref(&i.binding), Some(p), out)?;
    writeln!(
        out,
        "<g id=\"svg-{}-group\" role=\"group\" aria-label=\"{}\"><title>{}</title>",
        xml_attribute(&i.id)?,
        xml_attribute(&i.label.full_text)?,
        xml_attribute(&i.label.full_text)?
    )
    .expect("writing to String cannot fail");
    writeln!(out, "<path id=\"svg-{}-path\" d=\"{}\" style=\"fill:none;stroke:#17202a;stroke-width:2\" marker-end=\"url(#arrow-assertion)\"/>", xml_attribute(&i.id)?, path_data(&i.points)).expect("writing to String cannot fail");
    render_label_box(
        &format!("svg-{}-label", i.id),
        &i.label,
        &i.text,
        "interaction-label",
        out,
    )?;
    out.push_str("</g>\n");
    if linked {
        out.push_str("</a>\n");
    }
    Ok(())
}
