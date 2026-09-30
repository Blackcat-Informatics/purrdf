// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **A data graph's `sh:shapesGraph` links are part of the shapes graph that validates it.**
//!
//! SHACL 1.2 Core §6.4: "Every value of sh:shapesGraph is an IRI representing a graph that
//! SHOULD be included into the shapes graph used to validate the data graph. The value of
//! sh:shapesGraph may be a value of owl:versionIRI, so the same strategy of resolving a shapes
//! graph IRI from a version IRI, described for Shapes Graphs, applies here." PurRDF reads the
//! SHOULD as a MUST, through the same import table the shapes graph's `owl:imports` resolve
//! through, on every entry point the Python, WebAssembly and C hosts call.
//!
//! Every accepting case below has an observing oracle: the linked graph's shape reports a
//! result (`ex:Focus` has no `ex:p`, a `sh:minCount` violation) that the supplied shapes graph
//! alone cannot report, and the supplied shapes graph's own shape (`ex:Focus` must have no
//! `ex:q`, a `sh:maxCount` violation) keeps reporting beside it — so a run that silently
//! dropped the link, or replaced the shapes graph with it, reads differently from one that
//! unioned the two.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_shapes::engine::{self, ValidationOptions};
use purrdf_shapes::text_ingest::parse_ntriples_to_dataset;
use purrdf_shapes::{ShapesError, ShapesImportError, ShapesImports};
use purrdf_validate::{
    SarifOptions, pack_shapes_product, validate_changes_to_sarif_string, validate_to_sarif_string,
    validate_with_rebuilt_shapes_product, validate_with_shapes_product,
};

use purrdf_iri::vocab::owl::NS as OWL;
use purrdf_iri::vocab::rdf::TYPE as RDF_TYPE;
use purrdf_iri::vocab::sh::NS as SH;

/// The data graph's own IRI in every fixture.
const DATA_GRAPH: &str = "http://example.org/myDataGraph";
/// The two linked graphs of the specification's example.
const SHAPES1: &str = "http://example.org/graph-shapes1";
const SHAPES2: &str = "http://example.org/graph-shapes2";

/// The supplied shapes graph: `ex:Focus` must have no `ex:q` (it has one).
const LOCAL_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
    @prefix ex: <http://example.org/> .\n\
    ex:LocalNode a sh:NodeShape ; sh:targetNode ex:Focus ; sh:property ex:LocalShape .\n\
    ex:LocalShape sh:path ex:q ; sh:maxCount 0 .\n";

/// A linked shapes document whose property shape `name` requires `ex:Focus` to have
/// `path`.
fn linked_document(name: &str, path: &str, extra: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix ex: <http://example.org/> .\n\
         ex:{name}Node a sh:NodeShape ; sh:targetNode ex:Focus ; sh:property ex:{name} .\n\
         ex:{name} sh:path ex:{path} ; sh:minCount 1 .\n\
         {extra}"
    )
}

/// `ex:Focus ex:q "x"`, plus `rows` (N-Triples).
fn data(rows: &str) -> String {
    format!("<http://example.org/Focus> <http://example.org/q> \"x\" .\n{rows}")
}

/// `<subject> a sh:DataGraph`.
fn typed_data_graph(subject: &str) -> String {
    format!("<{subject}> <{RDF_TYPE}> <{SH}DataGraph> .\n")
}

/// `<subject> sh:shapesGraph <target>`.
fn link(subject: &str, target: &str) -> String {
    format!("<{subject}> <{SH}shapesGraph> <{target}> .\n")
}

/// `data` linking `targets` from the `sh:DataGraph` [`DATA_GRAPH`].
fn linking(targets: &[&str]) -> String {
    let mut rows = typed_data_graph(DATA_GRAPH);
    for target in targets {
        rows.push_str(&link(DATA_GRAPH, target));
    }
    data(&rows)
}

/// The source shapes of every result of validating `data_nt` against `shapes_ttl`.
fn fired(
    shapes_ttl: &str,
    data_nt: &str,
    table: &[(&str, &str)],
) -> Result<BTreeSet<String>, ShapesError> {
    let imports = ShapesImports::from_turtle(table)?;
    let report = engine::validate_graphs_with_options(
        data_nt,
        shapes_ttl,
        None,
        &ValidationOptions::default(),
        &imports,
    )?;
    Ok(report
        .results
        .iter()
        .map(|result| result.source_shape.to_string())
        .collect())
}

/// The shape names, as the report renders its source shapes.
fn shapes(names: &[&str]) -> BTreeSet<String> {
    names
        .iter()
        .map(|name| format!("<http://example.org/{name}>"))
        .collect()
}

#[track_caller]
fn assert_import_error(error: &ShapesError, expected: &ShapesImportError) {
    assert_eq!(error.as_imports(), Some(expected), "got: {error}");
}

/// A link nothing supplies is refused by name, on the engine, the SARIF boundary and the
/// change path alike — and the neighbouring call that supplies the graph validates.
#[test]
fn an_unsupplied_link_is_refused_by_name() {
    let expected = ShapesImportError::UnresolvedLink {
        iris: vec![SHAPES1.to_owned()],
    };
    let linked = linking(&[SHAPES1]);
    assert_import_error(
        &fired(LOCAL_SHAPES, &linked, &[]).expect_err("refused"),
        &expected,
    );
    assert_import_error(
        &validate_to_sarif_string(LOCAL_SHAPES, None, &linked, &SarifOptions::default(), &[])
            .expect_err("refused"),
        &expected,
    );
    assert_eq!(expected.kind(), "unresolved-shapes-graph-link");
    assert_eq!(expected.iris(), [SHAPES1]);
    assert!(expected.to_string().contains(&format!("<{SHAPES1}>")));

    let document = linked_document("LinkedShape", "p", "");
    let table: &[(&str, &str)] = &[(SHAPES1, &document)];
    validate_to_sarif_string(LOCAL_SHAPES, None, &linked, &SarifOptions::default(), table)
        .expect("the supplied link validates");
}

/// A supplied link is unioned into the shapes graph: its shape fires on the data beside the
/// supplied shapes graph's own, and without the link only the supplied shape fires. The data
/// graph's own `owl:imports` stays unenacted (Core §6.2) throughout.
#[test]
fn a_supplied_link_is_unioned_and_its_shape_fires() {
    let document = linked_document("LinkedShape", "p", "");
    let table: &[(&str, &str)] = &[(SHAPES1, &document)];
    let unenacted =
        format!("<{DATA_GRAPH}> <{OWL}imports> <http://example.org/never-supplied> .\n");
    let linked = format!("{}{unenacted}", linking(&[SHAPES1]));

    assert_eq!(
        fired(LOCAL_SHAPES, &linked, table).expect("linked"),
        shapes(&["LinkedShape", "LocalShape"]),
    );
    // The control: the same data with no link, and so no table entry to supply.
    let unlinked = data(&format!("{}{unenacted}", typed_data_graph(DATA_GRAPH)));
    assert_eq!(
        fired(LOCAL_SHAPES, &unlinked, &[]).expect("unlinked"),
        shapes(&["LocalShape"]),
    );
    // The SARIF boundary every non-Rust host calls reports the linked shape too.
    let sarif =
        validate_to_sarif_string(LOCAL_SHAPES, None, &linked, &SarifOptions::default(), table)
            .expect("sarif");
    assert!(sarif.contains("LinkedShape"), "{sarif}");
    assert!(sarif.contains("MinCountConstraintComponent"), "{sarif}");
}

/// A `sh:shapesGraph` on a node that is neither the data graph's loaded IRI nor a
/// `sh:DataGraph` is data: nothing is refused, nothing is unioned — a table entry for it is
/// unreached — and the triple stays in the data graph for a shape to read.
#[test]
fn a_link_on_a_non_anchor_node_is_data() {
    let rows = link("http://example.org/someNode", SHAPES1);
    let unanchored = data(&rows);
    assert_eq!(
        fired(LOCAL_SHAPES, &unanchored, &[]).expect("data, not a link"),
        shapes(&["LocalShape"]),
    );
    let document = linked_document("LinkedShape", "p", "");
    assert_import_error(
        &fired(LOCAL_SHAPES, &unanchored, &[(SHAPES1, &document)]).expect_err("unreached"),
        &ShapesImportError::Unreached {
            iris: vec![SHAPES1.to_owned()],
            unanchored: vec![],
        },
    );
    // The triple is ordinary data: a shape targeting its subject sees it.
    let reads_it = format!(
        "{LOCAL_SHAPES}ex:SeesItNode a sh:NodeShape ; sh:targetNode ex:someNode ;\n\
           sh:property ex:SeesIt .\n\
         ex:SeesIt sh:path sh:shapesGraph ; sh:maxCount 0 .\n"
    );
    assert_eq!(
        fired(&reads_it, &unanchored, &[]).expect("data"),
        shapes(&["LocalShape", "SeesIt"]),
    );
}

/// A link may name a VERSION IRI: a table entry under the version IRI resolves it, and the
/// ontology declaring that version has its own `owl:imports` followed; a version IRI the
/// supplied shapes graph declares resolves the link in place, with no table.
#[test]
fn a_link_via_version_iri_resolves() {
    let version = "http://example.org/graph-shapes1/v2";
    let base_shapes = "http://example.org/base-shapes";
    let versioned = linked_document(
        "LinkedShape",
        "p",
        &format!("<{SHAPES1}> owl:versionIRI <{version}> ; owl:imports <{base_shapes}> .\n"),
    );
    let imported = linked_document("BaseShape", "r", "");
    let linked = linking(&[version]);
    assert_eq!(
        fired(
            LOCAL_SHAPES,
            &linked,
            &[(version, &versioned), (base_shapes, &imported)]
        )
        .expect("resolved through the version IRI"),
        shapes(&["BaseShape", "LinkedShape", "LocalShape"]),
    );
    // The version's ontology's import is enacted: without its document, refused by name.
    assert_import_error(
        &fired(LOCAL_SHAPES, &linked, &[(version, &versioned)]).expect_err("import missing"),
        &ShapesImportError::Unresolved {
            iris: vec![base_shapes.to_owned()],
        },
    );
    // In place: the supplied shapes graph names the version, so no table is needed.
    let declaring = format!("{LOCAL_SHAPES}<{SHAPES1}> <{OWL}versionIRI> <{version}> .\n");
    assert_eq!(
        fired(&declaring, &linked, &[]).expect("resolved in place"),
        shapes(&["LocalShape"]),
    );
}

/// The linked graph's own `owl:imports` are imports: it was loaded under the link's IRI, so
/// an import on that IRI is followed — refused unsupplied, applied supplied.
#[test]
fn a_linked_graphs_own_imports_are_followed() {
    let lib = "http://example.org/lib2";
    let document = linked_document(
        "LinkedShape",
        "p",
        &format!("<{SHAPES1}> owl:imports <{lib}> .\n"),
    );
    let imported = linked_document("ImportedShape", "r", "");
    let linked = linking(&[SHAPES1]);
    assert_eq!(
        fired(
            LOCAL_SHAPES,
            &linked,
            &[(SHAPES1, &document), (lib, &imported)]
        )
        .expect("the closure resolves"),
        shapes(&["ImportedShape", "LinkedShape", "LocalShape"]),
    );
    assert_import_error(
        &fired(LOCAL_SHAPES, &linked, &[(SHAPES1, &document)]).expect_err("refused"),
        &ShapesImportError::Unresolved {
            iris: vec![lib.to_owned()],
        },
    );
}

/// The specification's example: two links, both unioned; one of them missing is refused
/// naming only it.
#[test]
fn two_links_are_unioned() {
    let first = linked_document("LinkedShape", "p", "");
    let second = linked_document("SecondShape", "r", "");
    let linked = linking(&[SHAPES1, SHAPES2]);
    assert_eq!(
        fired(
            LOCAL_SHAPES,
            &linked,
            &[(SHAPES1, &first), (SHAPES2, &second)]
        )
        .expect("both links resolve"),
        shapes(&["LinkedShape", "LocalShape", "SecondShape"]),
    );
    assert_import_error(
        &fired(LOCAL_SHAPES, &linked, &[(SHAPES1, &first)]).expect_err("one is missing"),
        &ShapesImportError::UnresolvedLink {
            iris: vec![SHAPES2.to_owned()],
        },
    );
}

/// With an empty shapes document the links ARE the shapes graph: the linked shape fires, and
/// the same empty document with no link reports nothing.
#[test]
fn an_empty_shapes_document_validates_against_the_linked_graphs() {
    let document = linked_document("LinkedShape", "p", "");
    assert_eq!(
        fired("", &linking(&[SHAPES1]), &[(SHAPES1, &document)]).expect("linked"),
        shapes(&["LinkedShape"]),
    );
    assert_eq!(
        fired("", &data(&typed_data_graph(DATA_GRAPH)), &[]).expect("unlinked"),
        BTreeSet::new(),
    );
}

/// A link value that is not an IRI names no graph and is refused; the same literal on a node
/// that is not a data-graph anchor is data and validates.
#[test]
fn a_non_iri_link_is_refused_and_non_anchor_literal_is_data() {
    let literal = format!("<{DATA_GRAPH}> <{SH}shapesGraph> \"{SHAPES1}\" .\n");
    let error = fired(
        LOCAL_SHAPES,
        &data(&format!("{}{literal}", typed_data_graph(DATA_GRAPH))),
        &[],
    )
    .expect_err("refused");
    assert_import_error(
        &error,
        &ShapesImportError::InvalidLink {
            values: vec![format!("\"{SHAPES1}\"")],
        },
    );
    assert_eq!(
        error.as_imports().map(ShapesImportError::kind),
        Some("invalid-shapes-graph-link")
    );
    assert_eq!(
        fired(LOCAL_SHAPES, &data(&literal), &[]).expect("data"),
        shapes(&["LocalShape"]),
    );
}

/// The data graph's loaded IRI anchors a link with no `sh:DataGraph` type, for a host that
/// knows it.
#[test]
fn the_data_graphs_loaded_iri_anchors_a_link() {
    let rows = data(&link(DATA_GRAPH, SHAPES1));
    let dataset = parse_ntriples_to_dataset(&rows).expect("data");
    let mut table = ShapesImports::new();
    table
        .link_data_graph(dataset.as_ref(), &[])
        .expect("no anchor");
    assert_eq!(table.links(), [] as [&str; 0]);
    table
        .link_data_graph(dataset.as_ref(), &[DATA_GRAPH])
        .expect("anchored");
    assert_eq!(table.links(), [SHAPES1]);
}

/// The change path reads the links off the MUTATED graph: a link the change adds is
/// resolved and unioned, and refused when nothing supplies it. A moved link changes the
/// shapes graph, so the run is a full validation that says why — `ex:Focus` is not a subject
/// of the change, and a bounded expansion would never have reached the linked shape's
/// result — while a change that moves no link stays bounded.
#[test]
fn a_link_a_change_adds_is_followed() {
    let base = data(&typed_data_graph(DATA_GRAPH));
    let added = link(DATA_GRAPH, SHAPES1);
    let document = linked_document("LinkedShape", "p", "");
    let options = SarifOptions::default();
    let (_, unmoved) = validate_changes_to_sarif_string(
        LOCAL_SHAPES,
        None,
        &base,
        Some("<http://example.org/Other> <http://example.org/q> \"y\" .\n"),
        None,
        &options,
        &[],
    )
    .expect("a change that moves no link validates");
    assert!(unmoved.is_bounded(), "{unmoved:?}");
    let (sarif, scope) = validate_changes_to_sarif_string(
        LOCAL_SHAPES,
        None,
        &base,
        Some(&added),
        None,
        &options,
        &[(SHAPES1, &document)],
    )
    .expect("the linked change validates");
    assert!(sarif.contains("LinkedShape"), "{sarif}");
    assert!(
        scope
            .reason()
            .is_some_and(|reason| reason.contains("sh:shapesGraph")),
        "{scope:?}"
    );
    let error = validate_changes_to_sarif_string(
        LOCAL_SHAPES,
        None,
        &base,
        Some(&added),
        None,
        &options,
        &[],
    )
    .expect_err("refused");
    assert_import_error(
        &error,
        &ShapesImportError::UnresolvedLink {
            iris: vec![SHAPES1.to_owned()],
        },
    );
    // Removing the link removes the requirement: the retraction validates with no table.
    let (sarif, retracted) = validate_changes_to_sarif_string(
        LOCAL_SHAPES,
        None,
        &linking(&[SHAPES1]),
        None,
        Some(&added),
        &options,
        &[],
    )
    .expect("the retracted link needs nothing");
    assert!(!sarif.contains("LinkedShape"), "{sarif}");
    assert!(sarif.contains("LocalShape"), "{sarif}");
    assert!(!retracted.is_bounded(), "{retracted:?}");
}

/// A shapes graph built before the data graph was known cannot take a link in, so every
/// validation checks each link is one it HOLDS: a graph its closure folded in, its base, or
/// a shapes graph it declares validates; any other link is refused naming it.
#[test]
fn a_prepared_shapes_graph_must_hold_every_link() {
    let lib = "http://example.org/lib";
    let declared = "http://example.org/declared";
    let base = "http://example.org/shapes";
    let importer = format!(
        "{LOCAL_SHAPES}<{base}> <{OWL}imports> <{lib}> .\n\
         <{declared}> <{RDF_TYPE}> <{SH}ShapesGraph> .\n"
    );
    let lib_document = linked_document("LibShape", "p", "");
    let imports = ShapesImports::from_turtle(&[(lib, &lib_document)]).expect("table");
    let prepared = Arc::new(
        engine::parse_shapes_with_config(&importer, Some(base), None, &imports).expect("parsed"),
    );
    for held in [lib, declared, base] {
        let report = engine::validate_dataset(
            parse_ntriples_to_dataset(&linking(&[held]))
                .expect("data")
                .as_ref(),
            &prepared,
        )
        .unwrap_or_else(|error| panic!("{held} is held: {error}"));
        let fired: BTreeSet<String> = report
            .results
            .iter()
            .map(|result| result.source_shape.to_string())
            .collect();
        assert_eq!(fired, shapes(&["LibShape", "LocalShape"]), "{held}");
    }
    let unheld = ShapesImportError::UnheldLink {
        iris: vec![SHAPES1.to_owned()],
    };
    let error = engine::validate_dataset(
        parse_ntriples_to_dataset(&linking(&[SHAPES1]))
            .expect("data")
            .as_ref(),
        &prepared,
    )
    .expect_err("not held");
    assert_import_error(&error, &unheld);
    assert_eq!(unheld.kind(), "unheld-shapes-graph-link");
    assert!(
        error.to_string().contains(&format!("<{SHAPES1}>")),
        "{error}"
    );
    let binding = engine::PreparedShapes::new(Arc::clone(&prepared)).bind_dataset(
        parse_ntriples_to_dataset(&linking(&[SHAPES1]))
            .expect("data")
            .as_ref(),
    );
    assert_import_error(
        &binding.expect_err("a binding checks the links too"),
        &unheld,
    );
}

/// A prepared PRODUCT records every graph its shapes graph absorbed by name, so it holds
/// a link to a table-supplied document with no ontology header of its own — the merged
/// dataset alone could never say that document was folded in — as well as its base and
/// the shapes graphs it declares, on the admit and rebuild paths alike. The linked shape's
/// result is in the report. A product packed WITHOUT that document refuses the same data
/// by name, typed.
#[test]
fn a_product_holds_every_graph_it_absorbed() {
    let lib = "http://example.org/lib";
    let declared = "http://example.org/declared";
    let base = "http://example.org/shapes";
    let importer = format!(
        "{LOCAL_SHAPES}<{base}> <{OWL}imports> <{lib}> .\n\
         <{declared}> <{RDF_TYPE}> <{SH}ShapesGraph> .\n"
    );
    // Header-less: the document declares no `owl:Ontology` or `sh:ShapesGraph` for its own
    // IRI, so only the import table knows it is `lib`.
    let lib_document = linked_document("LibShape", "p", "");
    assert!(!lib_document.contains("Ontology") && !lib_document.contains("ShapesGraph"));
    let table: &[(&str, &str)] = &[(lib, &lib_document)];
    let product = pack_shapes_product(&importer, Some(base), table).expect("packed");
    let options = SarifOptions::default();
    for held in [lib, declared, base] {
        for (lane, sarif) in [
            (
                "admit",
                validate_with_shapes_product(&product, &linking(&[held]), &options),
            ),
            (
                "rebuild",
                validate_with_rebuilt_shapes_product(&product, &linking(&[held]), &options),
            ),
        ] {
            let sarif = sarif.unwrap_or_else(|error| panic!("{lane}: {held} is held: {error}"));
            assert!(sarif.contains("LibShape"), "{lane} {held}: {sarif}");
            assert!(sarif.contains("LocalShape"), "{lane} {held}: {sarif}");
        }
    }
    let unheld = ShapesImportError::UnheldLink {
        iris: vec![SHAPES1.to_owned()],
    };
    let refused = validate_with_shapes_product(&product, &linking(&[SHAPES1]), &options)
        .expect_err("not held");
    assert_eq!(refused.import_error(), Some(&unheld), "{refused}");

    // The neighbour that lacks the document: packed from the supplied shapes alone, it
    // holds no `lib`, and the same data is refused naming it.
    let bare = pack_shapes_product(LOCAL_SHAPES, Some(base), &[]).expect("packed");
    let refused = validate_with_shapes_product(&bare, &linking(&[lib]), &options)
        .expect_err("the bare product does not hold lib");
    assert_eq!(
        refused.import_error(),
        Some(&ShapesImportError::UnheldLink {
            iris: vec![lib.to_owned()],
        }),
        "{refused}"
    );
    let refused = validate_with_rebuilt_shapes_product(&bare, &linking(&[lib]), &options)
        .expect_err("nor does its rebuild");
    assert_eq!(
        refused.import_error().map(ShapesImportError::kind),
        Some("unheld-shapes-graph-link"),
        "{refused}"
    );
}
