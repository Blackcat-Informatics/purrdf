// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Generate deterministic RDF 1.2 visualization acceptance artifacts.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use purrdf_rdf::viz::{
    VizGraphInput, VizInputAnnotation, VizInputQuad, VizInputReifier, VizInputStatement,
    VizLayoutOptions, VizMode, VizRenderOptions, VizSpec, VizSvgOptions, VizTableField,
    VizVocabularyMapping, export_json, render_graph_input_svg, stable_hash_hex,
};
use purrdf_rdf::{RdfTextDirection, TermValue};

const EX: &str = "https://example.org/";
const PROV: &str = "http://www.w3.org/ns/prov#";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("/inbox"), PathBuf::from);
    fs::create_dir_all(&output)?;
    let fixtures = [
        ("ordinary-shared", ordinary_shared()),
        ("asserted-reified", asserted_reified()),
        ("quoted-only", quoted_only()),
        ("nested-dialect", nested_dialect()),
        ("dense-connected", dense_connected()),
    ];
    let mut summary = String::from(
        "PurRDF RDF 1.2 visualization acceptance artifacts\n\n\
         Each SVG is produced through purrdf_rdf::viz from graph-like Rust input.\n\n",
    );
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>PurRDF RDF 1.2 visualization samples</title>\
         <style>body{margin:0;background:#e2e8f0;color:#17202a;font:14px system-ui}h1,h2{margin:20px}section{margin:20px;background:white;border:1px solid #94a3b8}object{display:block;width:100%;min-height:520px;border-top:1px solid #cbd5e1}</style>\
         <h1>PurRDF RDF 1.2 visualization samples</h1>",
    );
    for (name, input) in fixtures {
        for mode in [VizMode::Compact, VizMode::Incidence, VizMode::Table] {
            let mode_name = mode_name(mode);
            let spec = VizSpec {
                mode,
                vocabulary: vec![
                    VizVocabularyMapping {
                        prefix: "ex".to_owned(),
                        namespace: EX.to_owned(),
                    },
                    VizVocabularyMapping {
                        prefix: "prov".to_owned(),
                        namespace: PROV.to_owned(),
                    },
                ],
                table_fields: vec![
                    VizTableField::Statement,
                    VizTableField::AssertedIn,
                    VizTableField::Reifiers,
                    VizTableField::Annotations,
                    VizTableField::ReferencedBy,
                    VizTableField::Depth,
                    VizTableField::Diagnostics,
                ],
                max_statements: 1_000,
                max_terms: 3_000,
                ..VizSpec::default()
            };
            let options = VizRenderOptions {
                layout: VizLayoutOptions {
                    rank_spacing: 180,
                    node_spacing: 54,
                    component_spacing: 110,
                    component_wrap_width: 1_900,
                    crossing_sweeps: 12,
                    max_node_width: 320,
                    ..VizLayoutOptions::default()
                },
                svg: VizSvgOptions {
                    title: format!("PurRDF RDF 1.2 {name} {mode_name}"),
                    ..VizSvgOptions::default()
                },
            };
            let document = render_graph_input_svg(&input, &spec, &options)?;
            let stem = format!("purrdf-viz2-{name}-{mode_name}");
            let svg_path = output.join(format!("{stem}.svg"));
            let json_path = output.join(format!("{stem}.json"));
            fs::write(&svg_path, &document.svg)?;
            fs::write(&json_path, export_json(&document.export)?)?;
            writeln!(
                summary,
                "{stem}: {}x{}, {} nodes, {} edges, {} statements, svg-fnv64 {}",
                document.export.layout.width,
                document.export.layout.height,
                document.export.scene.nodes.len(),
                document.export.scene.edges.len(),
                document.export.model.statements.len(),
                stable_hash_hex(&document.svg)
            )?;
            write!(
                html,
                "<section><h2>{name} / {mode_name}</h2><object data=\"{stem}.svg\" type=\"image/svg+xml\"></object></section>"
            )?;
        }
    }
    fs::write(output.join("purrdf-viz2-summary.txt"), summary)?;
    fs::write(output.join("purrdf-viz2-index.html"), html)?;
    println!(
        "wrote RDF 1.2 visualization artifacts to {}",
        output.display()
    );
    Ok(())
}

fn ordinary_shared() -> VizGraphInput {
    VizGraphInput {
        quads: vec![
            quad("alice", "knows", iri("bob"), "facts"),
            quad("alice", "worksWith", iri("carol"), "facts"),
            quad("bob", "worksWith", iri("carol"), "facts"),
            quad("carol", "reportsTo", iri("dana"), "facts"),
            quad("bob", "reportsTo", iri("dana"), "facts"),
            quad("dana", "mentors", iri("alice"), "facts"),
            quad("alice", "name", literal("Alice A."), "labels"),
            quad("bob", "name", literal("Bob B."), "labels"),
        ],
        ..VizGraphInput::default()
    }
}

fn asserted_reified() -> VizGraphInput {
    let knows = statement("alice", "knows", iri("bob"));
    let reviewed = statement("bob", "reviewed", iri("report-17"));
    VizGraphInput {
        quads: vec![
            quad("alice", "knows", iri("bob"), "facts"),
            quad("bob", "reviewed", iri("report-17"), "facts"),
            quad("report-17", "about", iri("project-aurora"), "records"),
            quad("carol", "audits", iri("report-17"), "audit"),
            quad("alice", "worksWith", iri("carol"), "facts"),
        ],
        reifiers: vec![
            VizInputReifier {
                reifier: iri("claim-primary"),
                statement: knows.clone(),
                graph_name: Some(iri("claims")),
            },
            VizInputReifier {
                reifier: iri("observation-42"),
                statement: knows,
                graph_name: Some(iri("observations")),
            },
            VizInputReifier {
                reifier: iri("claim-primary"),
                statement: reviewed,
                graph_name: Some(iri("claims")),
            },
        ],
        annotations: vec![
            annotation("claim-primary", "confidence", decimal("0.82"), "provenance"),
            annotation(
                "claim-primary",
                "wasAttributedTo",
                iri("carol"),
                "provenance",
            ),
            annotation("claim-primary", "sourceRecord", iri("report-17"), "records"),
            annotation(
                "observation-42",
                "observedAt",
                literal("2026-07-10T08:30:00Z"),
                "observations",
            ),
            annotation(
                "observation-42",
                "message",
                directional("مرحبا من إدمونتون", "ar", RdfTextDirection::Rtl),
                "observations",
            ),
        ],
    }
}

fn quoted_only() -> VizGraphInput {
    VizGraphInput {
        reifiers: vec![
            VizInputReifier {
                reifier: iri("witness-account"),
                statement: statement("dave", "saw", iri("erin")),
                graph_name: Some(iri("testimony")),
            },
            VizInputReifier {
                reifier: iri("counterclaim"),
                statement: statement("dave", "didNotSee", iri("erin")),
                graph_name: Some(iri("testimony")),
            },
        ],
        annotations: vec![
            annotation(
                "witness-account",
                "wasAttributedTo",
                iri("frank"),
                "provenance",
            ),
            annotation(
                "witness-account",
                "confidence",
                decimal("0.61"),
                "provenance",
            ),
            annotation(
                "counterclaim",
                "wasAttributedTo",
                iri("grace"),
                "provenance",
            ),
        ],
        ..VizGraphInput::default()
    }
}

fn nested_dialect() -> VizGraphInput {
    let alice_knows_bob = triple("alice", "knows", iri("bob"));
    let nested_report = TermValue::Triple {
        s: Box::new(alice_knows_bob.clone()),
        p: Box::new(TermValue::Iri(format!("{EX}reportedBy"))),
        o: Box::new(iri("carol")),
    };
    VizGraphInput {
        quads: vec![
            VizInputQuad {
                subject: alice_knows_bob,
                predicate: format!("{EX}reportedBy"),
                object: iri("carol"),
                graph_name: Some(iri("reports")),
            },
            VizInputQuad {
                subject: iri("audit-node"),
                predicate: format!("{EX}checks"),
                object: nested_report,
                graph_name: Some(iri("audit")),
            },
            VizInputQuad {
                subject: literal("generalized subject"),
                predicate: format!("{EX}describes"),
                object: iri("extension-case"),
                graph_name: Some(iri("extensions")),
            },
        ],
        reifiers: vec![VizInputReifier {
            reifier: iri("meta-claim"),
            statement: statement("alice", "knows", iri("bob")),
            graph_name: Some(iri("claims")),
        }],
        annotations: vec![annotation(
            "meta-claim",
            "nestingNote",
            literal("inner proposition is quoted, not independently asserted"),
            "provenance",
        )],
    }
}

fn dense_connected() -> VizGraphInput {
    let mut quads = Vec::new();
    let mut reifiers = Vec::new();
    let mut annotations = Vec::new();
    let predicates = ["knows", "worksWith", "reviewed", "dependsOn", "cites"];
    for index in 0..28 {
        let source = format!("resource-{}", index % 14);
        let target = format!("resource-{}", (index * 5 + 3) % 14);
        let predicate = predicates[index % predicates.len()];
        quads.push(quad(
            &source,
            predicate,
            iri(&target),
            if index % 3 == 0 { "records" } else { "facts" },
        ));
        if index % 5 == 0 {
            let claim = format!("claim-{index}");
            reifiers.push(VizInputReifier {
                reifier: iri(&claim),
                statement: statement(&source, predicate, iri(&target)),
                graph_name: Some(iri("claims")),
            });
            annotations.push(annotation(
                &claim,
                "confidence",
                decimal(&format!("0.{}", 5 + index % 5)),
                "provenance",
            ));
            annotations.push(annotation(
                &claim,
                "wasAttributedTo",
                iri(&format!("agent-{}", index % 4)),
                "provenance",
            ));
        }
    }
    VizGraphInput {
        quads,
        reifiers,
        annotations,
    }
}

fn quad(subject: &str, predicate: &str, object: TermValue, graph: &str) -> VizInputQuad {
    VizInputQuad {
        subject: iri(subject),
        predicate: format!("{EX}{predicate}"),
        object,
        graph_name: Some(iri(graph)),
    }
}

fn statement(subject: &str, predicate: &str, object: TermValue) -> VizInputStatement {
    VizInputStatement {
        subject: iri(subject),
        predicate: format!("{EX}{predicate}"),
        object,
    }
}

fn annotation(
    reifier: &str,
    predicate: &str,
    object: TermValue,
    graph: &str,
) -> VizInputAnnotation {
    VizInputAnnotation {
        reifier: iri(reifier),
        predicate: if predicate == "wasAttributedTo" {
            format!("{PROV}{predicate}")
        } else {
            format!("{EX}{predicate}")
        },
        object,
        graph_name: Some(iri(graph)),
    }
}

fn triple(subject: &str, predicate: &str, object: TermValue) -> TermValue {
    TermValue::Triple {
        s: Box::new(iri(subject)),
        p: Box::new(TermValue::Iri(format!("{EX}{predicate}"))),
        o: Box::new(object),
    }
}

fn iri(local: &str) -> TermValue {
    TermValue::Iri(format!("{EX}{local}"))
}

fn literal(value: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: value.to_owned(),
        datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
        language: None,
        direction: None,
    }
}

fn decimal(value: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: value.to_owned(),
        datatype: "http://www.w3.org/2001/XMLSchema#decimal".to_owned(),
        language: None,
        direction: None,
    }
}

fn directional(value: &str, language: &str, direction: RdfTextDirection) -> TermValue {
    TermValue::Literal {
        lexical_form: value.to_owned(),
        datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".to_owned(),
        language: Some(language.to_owned()),
        direction: Some(direction),
    }
}

fn mode_name(mode: VizMode) -> &'static str {
    match mode {
        VizMode::Compact => "compact",
        VizMode::Incidence => "incidence",
        VizMode::Table => "table",
    }
}
