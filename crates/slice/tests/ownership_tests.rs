// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Hermetic acceptance tests for the native ownership + dependency analyzer
//! (RFC #820 §10 / S4). Each test builds minimal slice directories on disk
//! under a `tempfile::TempDir`, discovers them via [`SliceCatalog::discover`],
//! and asserts on the [`OwnershipReport`].

use std::path::Path;

use tempfile::TempDir;

use purrdf_slice::{
    EdgeKind, NamedNode, OwnershipAnalyzer, OwnershipDiagnostic, OwnershipStatus,
    ReconciliationStatus, SliceCatalog, SliceVocab,
};

/// Pure fixtures use a caller-supplied example.org vocabulary (the slice
/// vocabulary is never PurRDF's own).
const NS: &str = "https://example.org/vocab/";

fn test_vocab() -> SliceVocab {
    SliceVocab::for_namespace(NS)
}

// ── Fixture helpers ───────────────────────────────────────────────────────────

/// Write a file, creating parent directories as needed.
fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
}

/// A minimal valid `manifest.ttl` for a slice with the given IRI suffix and
/// optional `vocab:sliceDependsOn` targets (IRI suffixes).
fn manifest(slice: &str, depends_on: &[&str]) -> String {
    use std::fmt::Write as _;
    let mut deps = String::new();
    for d in depends_on {
        let _ = writeln!(deps, "    vocab:sliceDependsOn vocab:{d} ;");
    }
    format!(
        "@prefix vocab: <{NS}> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         @prefix dcterms: <http://purl.org/dc/terms/> .\n\n\
         vocab:{slice} a vocab:Slice ;\n\
         {deps}    rdfs:label \"{slice}\"@x-purrdf-english ;\n\
         dcterms:title \"{slice} slice\"@x-purrdf-english .\n"
    )
}

/// Build a full slice IRI from a local-name suffix.
fn iri(suffix: &str) -> String {
    format!("{NS}{suffix}")
}

fn nn(suffix: &str) -> NamedNode {
    NamedNode::new(iri(suffix)).unwrap()
}

// ── Test 1: single validated owner ────────────────────────────────────────────

#[test]
fn single_validated_owner() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Slice A defines term T via rdfs:isDefinedBy → sliceA.
    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    write(
        root,
        "slices/grpA/sliceA/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:termT a owl:Class ;\n\
             rdfs:isDefinedBy vocab:sliceA ;\n\
             rdfs:label \"term T\"@x-purrdf-english .\n"
        ),
    );

    // Slice B references term T in its module (a dependency on A).
    write(
        root,
        "slices/grpB/sliceB/manifest.ttl",
        &manifest("sliceB", &["sliceA"]),
    );
    write(
        root,
        "slices/grpB/sliceB/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:termU a owl:Class ;\n\
             rdfs:isDefinedBy vocab:sliceB ;\n\
             rdfs:subClassOf vocab:termT .\n"
        ),
    );

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();

    // Exactly ONE validated owner for T.
    let t = &report.ownership[&nn("termT")];
    assert_eq!(t.declared_owner, iri("sliceA"));
    assert_eq!(t.status, OwnershipStatus::Validated);
    assert!(t.physical_origin.is_some());
    assert_eq!(t.physical_origin.as_ref().unwrap().slice, iri("sliceA"));

    // No conflict / mismatch diagnostics.
    assert!(
        !report.has_ownership_defect(),
        "expected no ownership defect"
    );

    // B → A edge exists, semantic (Ontology), declared → Matched.
    let edge = report
        .edges
        .iter()
        .find(|e| e.from_slice == iri("sliceB") && e.to_slice == iri("sliceA"))
        .expect("expected a sliceB → sliceA edge");
    assert_eq!(edge.edge_kind, EdgeKind::Ontology);
    assert_eq!(edge.reconciliation, ReconciliationStatus::Matched);
    assert!(
        edge.evidence
            .iter()
            .any(|ev| ev.referenced_term == nn("termT")),
        "edge must carry termT as evidence"
    );
}

// ── Test 2: ownership conflict ────────────────────────────────────────────────

#[test]
fn ownership_conflict() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Both A and B claim isDefinedBy for term T.
    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    write(
        root,
        "slices/grpA/sliceA/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:termT a owl:Class ; rdfs:isDefinedBy vocab:sliceA .\n"
        ),
    );
    write(
        root,
        "slices/grpB/sliceB/manifest.ttl",
        &manifest("sliceB", &[]),
    );
    write(
        root,
        "slices/grpB/sliceB/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:termT a owl:Class ; rdfs:isDefinedBy vocab:sliceB .\n"
        ),
    );

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();

    let t = &report.ownership[&nn("termT")];
    match &t.status {
        OwnershipStatus::Conflict(claimants) => {
            assert!(claimants.contains(&iri("sliceA")));
            assert!(claimants.contains(&iri("sliceB")));
        }
        other => panic!("expected Conflict, got {other:?}"),
    }
    assert!(report.has_ownership_defect());
    assert!(report.diagnostics.iter().any(|d| matches!(
        d,
        OwnershipDiagnostic::Conflict { term, .. } if *term == nn("termT")
    )));
}

// ── Test 2b: ownership mismatch (declared owner ≠ physical slice) ──────────────

#[test]
fn ownership_mismatch() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Slice A is the only physical slice, but its module declares term T as
    // owned by a DIFFERENT/foreign slice IRI (vocab:sliceElsewhere). The
    // declared owner therefore disagrees with the physical origin (sliceA).
    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    write(
        root,
        "slices/grpA/sliceA/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:termT a owl:Class ; rdfs:isDefinedBy vocab:sliceElsewhere .\n"
        ),
    );

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();

    let t = &report.ownership[&nn("termT")];
    match &t.status {
        OwnershipStatus::Mismatch { declared, physical } => {
            assert_eq!(*declared, iri("sliceElsewhere"));
            assert_eq!(*physical, iri("sliceA"));
        }
        other => panic!("expected Mismatch, got {other:?}"),
    }
    assert!(report.has_ownership_defect());
    assert!(report.diagnostics.iter().any(|d| matches!(
        d,
        OwnershipDiagnostic::Mismatch { term, declared, physical }
            if *term == nn("termT")
                && *declared == iri("sliceElsewhere")
                && *physical == iri("sliceA")
    )));
}

// ── Test 3: parsed (not textual) edges ────────────────────────────────────────

#[test]
fn parsed_not_textual_edges() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Slice A owns termT.
    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    write(
        root,
        "slices/grpA/sliceA/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:termT a owl:Property ; rdfs:isDefinedBy vocab:sliceA .\n\
             vocab:termGhost a owl:Property ; rdfs:isDefinedBy vocab:sliceA .\n"
        ),
    );

    // Slice B has a SPARQL query that:
    //  - uses vocab:termT as a predicate (a real term reference), and
    //  - mentions vocab:termGhost ONLY inside a string literal (must NOT count).
    write(
        root,
        "slices/grpB/sliceB/manifest.ttl",
        &manifest("sliceB", &["sliceA"]),
    );
    write(
        root,
        "slices/grpB/sliceB/queries/competency/q.rq",
        &format!(
            "PREFIX vocab: <{NS}>\n\
             SELECT ?s ?label WHERE {{\n\
             ?s vocab:termT ?o .\n\
             BIND(\"see <{NS}termGhost> for details\" AS ?label)\n\
             }}\n"
        ),
    );

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();

    // Exactly one B → A query edge, with termT as evidence.
    let edge = report
        .edges
        .iter()
        .find(|e| {
            e.from_slice == iri("sliceB")
                && e.to_slice == iri("sliceA")
                && e.edge_kind == EdgeKind::Query
        })
        .expect("expected a sliceB → sliceA Query edge");

    let referenced: Vec<&NamedNode> = edge.evidence.iter().map(|e| &e.referenced_term).collect();
    assert!(
        referenced.contains(&&nn("termT")),
        "termT (a parsed predicate) must be evidence"
    );
    assert!(
        !referenced.contains(&&nn("termGhost")),
        "termGhost (only in a string literal) must NOT be evidence"
    );
}

// ── Test 4: path independence ─────────────────────────────────────────────────

#[test]
fn path_independence() {
    // Build the SAME logical slice content at two different filesystem layouts.
    fn build(root: &Path, group: &str) {
        write(
            root,
            &format!("slices/{group}/sliceA/manifest.ttl"),
            &manifest("sliceA", &[]),
        );
        write(
            root,
            &format!("slices/{group}/sliceA/module.ttl"),
            &format!(
                "@prefix vocab: <{NS}> .\n\
                 @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                 @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
                 vocab:termT a owl:Class ; rdfs:isDefinedBy vocab:sliceA .\n"
            ),
        );
    }

    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();
    build(tmp1.path(), "core");
    build(tmp2.path(), "extensions/deeply/nested");

    let r1 = OwnershipAnalyzer::new(&SliceCatalog::discover(tmp1.path(), test_vocab()).unwrap())
        .analyze()
        .unwrap();
    let r2 = OwnershipAnalyzer::new(&SliceCatalog::discover(tmp2.path(), test_vocab()).unwrap())
        .analyze()
        .unwrap();

    // Ownership tables are identical (term IRIs, owners, status — all
    // path-independent). physical_origin.logical_path is slice-relative, so it
    // too is identical across the differing group paths.
    let o1 = &r1.ownership[&nn("termT")];
    let o2 = &r2.ownership[&nn("termT")];
    assert_eq!(o1.declared_owner, o2.declared_owner);
    assert_eq!(o1.status, o2.status);
    assert_eq!(o1.physical_origin, o2.physical_origin);
    assert_eq!(o1, o2);
}

// ── Test 5: semantic vs non-semantic edges ────────────────────────────────────

#[test]
fn semantic_vs_nonsemantic() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Slice A owns termT.
    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    write(
        root,
        "slices/grpA/sliceA/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:termT a owl:Class ; rdfs:isDefinedBy vocab:sliceA .\n"
        ),
    );

    // Slice B references termT ONLY in documentation (non-semantic) — and does
    // NOT declare sliceDependsOn. A documentation cross-ref must NOT become a
    // reconcilable build dependency.
    write(
        root,
        "slices/grpB/sliceB/manifest.ttl",
        &manifest("sliceB", &[]),
    );
    write(
        root,
        "slices/grpB/sliceB/docs.md",
        &format!("# Slice B\nSee [termT]({NS}termT) in slice A.\n"),
    );

    // Slice C references termT in its ontology module (semantic) but does NOT
    // declare sliceDependsOn → an Undeclared semantic edge.
    write(
        root,
        "slices/grpC/sliceC/manifest.ttl",
        &manifest("sliceC", &[]),
    );
    write(
        root,
        "slices/grpC/sliceC/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:termC a owl:Class ; rdfs:isDefinedBy vocab:sliceC ;\n\
             rdfs:subClassOf vocab:termT .\n"
        ),
    );

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();

    // The documentation edge B → A (if recorded at all) is NON-semantic and
    // never reconciles / never produces an UndeclaredDependency diagnostic.
    if let Some(doc_edge) = report
        .edges
        .iter()
        .find(|e| e.from_slice == iri("sliceB") && e.to_slice == iri("sliceA"))
    {
        assert_eq!(doc_edge.edge_kind, EdgeKind::Documentation);
        assert!(!doc_edge.edge_kind.is_semantic());
        // Non-semantic edges carry Undeclared (evidence-only), never Matched.
        assert_ne!(doc_edge.reconciliation, ReconciliationStatus::Matched);
    }
    // No UndeclaredDependency diagnostic for the documentation edge.
    assert!(
        !report.diagnostics.iter().any(|d| matches!(
            d,
            OwnershipDiagnostic::UndeclaredDependency { from_slice, to_slice, .. }
                if *from_slice == iri("sliceB") && *to_slice == iri("sliceA")
        )),
        "a documentation cross-ref must not yield an UndeclaredDependency"
    );

    // The ontology edge C → A IS semantic and, being undeclared, yields an
    // UndeclaredDependency diagnostic.
    let onto_edge = report
        .edges
        .iter()
        .find(|e| {
            e.from_slice == iri("sliceC")
                && e.to_slice == iri("sliceA")
                && e.edge_kind == EdgeKind::Ontology
        })
        .expect("expected a sliceC → sliceA Ontology edge");
    assert!(onto_edge.edge_kind.is_semantic());
    assert_eq!(onto_edge.reconciliation, ReconciliationStatus::Undeclared);
    assert!(report.diagnostics.iter().any(|d| matches!(
        d,
        OwnershipDiagnostic::UndeclaredDependency { from_slice, to_slice, .. }
            if *from_slice == iri("sliceC") && *to_slice == iri("sliceA")
    )));
}

// ── Test 6: declared-but-unowned term ────────────────────────────────────────
//
// A term typed as owl:Class in a slice's module.ttl but with NO
// rdfs:isDefinedBy must yield OwnershipStatus::Unowned + an
// OwnershipDiagnostic::Unowned, and must NOT prevent Validated terms from
// being so (regression guard for the dead-code fix in Phase 2).

#[test]
fn declared_but_unowned_term() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Slice A: termWithOwner has rdfs:isDefinedBy → sliceA (Validated).
    //          termNoOwner is an owl:Class but has NO rdfs:isDefinedBy (Unowned).
    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    // Use a helper string to avoid Turtle string-literal double-quotes inside
    // a Rust format string (the @x-purrdf-english lang-tag follows the closing
    // quote of the Turtle literal and would close the Rust string early).
    let module_ttl = format!(
        concat!(
            "@prefix vocab: <{ns}> .\n",
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n",
            "vocab:termWithOwner a owl:Class ;\n",
            "    rdfs:isDefinedBy vocab:sliceA ;\n",
            "    rdfs:label \"owned\"@x-purrdf-english .\n\n",
            "vocab:termNoOwner a owl:Class ;\n",
            "    rdfs:label \"orphan\"@x-purrdf-english .\n",
        ),
        ns = NS,
    );
    write(root, "slices/grpA/sliceA/module.ttl", &module_ttl);

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();

    // termWithOwner must be Validated.
    let owned = &report.ownership[&nn("termWithOwner")];
    assert_eq!(
        owned.status,
        OwnershipStatus::Validated,
        "termWithOwner must be Validated"
    );

    // termNoOwner must be Unowned.
    let unowned = report
        .ownership
        .get(&nn("termNoOwner"))
        .expect("termNoOwner must appear in the ownership table even without rdfs:isDefinedBy");
    assert_eq!(
        unowned.status,
        OwnershipStatus::Unowned,
        "termNoOwner has no rdfs:isDefinedBy — must be Unowned"
    );
    assert!(
        unowned.declared_owner.is_empty(),
        "Unowned term has no declared owner"
    );
    assert!(
        unowned.physical_origin.is_none(),
        "Unowned term has no physical origin"
    );

    // An OwnershipDiagnostic::Unowned must be emitted for termNoOwner.
    assert!(
        report.diagnostics.iter().any(|d| matches!(
            d,
            OwnershipDiagnostic::Unowned { term } if *term == nn("termNoOwner")
        )),
        "expected an OwnershipDiagnostic::Unowned for termNoOwner"
    );

    // has_ownership_defect reflects the Unowned term.
    assert!(
        report.has_ownership_defect(),
        "report with an Unowned term must report has_ownership_defect"
    );
}

// ── Test 7: Group aggregate IRI is captured in slice dependency walk ─────────

#[test]
fn group_aggregate_iri_reaches_dependency_walk() {
    // G4-B regression guard: the previous Group arm in walk_graph_pattern only
    // walked `inner` and silently dropped the `aggregates` field.  A purrdf
    // extension-function IRI referenced ONLY inside an aggregate expression
    // therefore vanished from the dependency set, creating an invisible build-dep gap.
    //
    // This test places `vocab:heldIn` EXCLUSIVELY in the aggregated expression of a
    // GROUP query (`SUM(vocab:heldIn(?x))`).  After the fix the dependency edge must
    // carry `heldIn` as evidence; before the fix it would not, and the query edge
    // might not even appear. (A vocab: IRI in call position must be a real purrdf
    // extension function; the walker reconstructs its IRI from the closed set.)
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Slice A defines the term that is referenced only inside the aggregate.
    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    write(
        root,
        "slices/grpA/sliceA/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:heldIn a owl:ObjectProperty ; rdfs:isDefinedBy vocab:sliceA .\n"
        ),
    );

    // Slice B's query: grouped query where vocab:termAggFn appears ONLY inside
    // the aggregated expression of SUM — not as a predicate or IRI in the BGP.
    // Before the fix the Group arm dropped `aggregates` entirely.
    write(
        root,
        "slices/grpB/sliceB/manifest.ttl",
        &manifest("sliceB", &["sliceA"]),
    );
    write(
        root,
        "slices/grpB/sliceB/queries/competency/agg.rq",
        &format!(
            "PREFIX vocab: <{NS}>\n\
             SELECT ?t (SUM(vocab:heldIn(?x)) AS ?total) WHERE {{\n\
             ?x a ?t .\n\
             }} GROUP BY ?t\n"
        ),
    );

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();

    // A Query edge from B to A must exist.
    let edge = report
        .edges
        .iter()
        .find(|e| {
            e.from_slice == iri("sliceB")
                && e.to_slice == iri("sliceA")
                && e.edge_kind == EdgeKind::Query
        })
        .expect("expected a sliceB → sliceA Query edge; Group aggregate walk dropped it");

    // The IRI used only inside the aggregate expression must be evidence.
    assert!(
        edge.evidence
            .iter()
            .any(|e| e.referenced_term == nn("heldIn")),
        "heldIn (a purrdf extension function referenced only inside a Group aggregate \
         expression) must appear in evidence; walk_graph_pattern dropped Group aggregates \
         before the G4-B fix, and the walker must extract purrdf extension-function IRIs"
    );
}

// ── Consumer-fidelity guards: walker must not drop IRI-bearing positions ──────
//
// G12/G13(OrderBy)/G14: each builds sliceA (defines `defined_term`) and sliceB
// (a query that references it ONLY in the position under test), then asserts the
// B → A Query edge carries `defined_term` as evidence. Before the fix the walker
// dropped that position and the edge/evidence vanished.

/// Returns the evidence terms on the sliceB → sliceA Query edge for a query that
/// references `defined_term` (owned by sliceA). Panics if the edge is absent.
fn query_edge_evidence(query_body: &str, defined_term: &str) -> Vec<NamedNode> {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    write(
        root,
        "slices/grpA/sliceA/module.ttl",
        &format!(
            "@prefix vocab: <{NS}> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\n\
             vocab:{defined_term} a owl:ObjectProperty ; rdfs:isDefinedBy vocab:sliceA .\n"
        ),
    );
    write(
        root,
        "slices/grpB/sliceB/manifest.ttl",
        &manifest("sliceB", &["sliceA"]),
    );
    write(
        root,
        "slices/grpB/sliceB/queries/competency/q.rq",
        query_body,
    );

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();
    let edge = report
        .edges
        .iter()
        .find(|e| {
            e.from_slice == iri("sliceB")
                && e.to_slice == iri("sliceA")
                && e.edge_kind == EdgeKind::Query
        })
        .expect("expected a sliceB → sliceA Query edge; the walker dropped the dependency");
    edge.evidence
        .iter()
        .map(|e| e.referenced_term.clone())
        .collect()
}

#[test]
fn describe_target_iri_reaches_dependency_walk() {
    // G12: `DESCRIBE <iri>` stores the IRI in `targets`, and the pattern is the
    // empty unit pattern — walking only `pattern` dropped the edge.
    let ev = query_edge_evidence(
        &format!("PREFIX vocab: <{NS}>\nDESCRIBE vocab:describedThing\n"),
        "describedThing",
    );
    assert!(
        ev.contains(&nn("describedThing")),
        "DESCRIBE target IRI must be dependency evidence; got {ev:?}"
    );
}

#[test]
fn order_by_function_iri_reaches_dependency_walk() {
    // G13: a function IRI used only in an ORDER BY key was dropped because the
    // OrderBy arm matched `..` and never walked `expression`. The IRI is the purrdf
    // extension function vocab:heldIn (a vocab: IRI in call position must be a real
    // purrdf extension function; the walker reconstructs its IRI from the closed set).
    let ev = query_edge_evidence(
        &format!(
            "PREFIX vocab: <{NS}>\nSELECT ?x WHERE {{ ?x vocab:p ?v }} ORDER BY DESC(vocab:heldIn(?v))\n"
        ),
        "heldIn",
    );
    assert!(
        ev.contains(&nn("heldIn")),
        "ORDER BY function IRI must be dependency evidence; got {ev:?}"
    );
}

#[test]
fn values_quoted_triple_iri_reaches_dependency_walk() {
    // G14: an IRI inside an RDF 1.2 ground quoted-triple VALUES cell was dropped
    // because the Values arm had no GroundTerm::Triple recursion.
    let ev = query_edge_evidence(
        &format!(
            "PREFIX vocab: <{NS}>\nSELECT ?t WHERE {{ VALUES ?t {{ <<( vocab:qs vocab:quotedPred vocab:qo )>> }} }}\n"
        ),
        "quotedPred",
    );
    assert!(
        ev.contains(&nn("quotedPred")),
        "predicate inside a quoted-triple VALUES cell must be dependency evidence; got {ev:?}"
    );
}

// ── Hard-fail on a malformed ownership-bearing artifact (G9, no-optionality) ───

#[test]
fn malformed_artifact_hard_fails_analysis() {
    // A slice whose module.ttl is malformed RDF must make analyze() return Err,
    // never silently drop the artifact (which would hide its terms from the
    // one-validated-owner gate). No-optionality / hard-fail doctrine (#820).
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    write(
        root,
        "slices/grpA/sliceA/manifest.ttl",
        &manifest("sliceA", &[]),
    );
    // Broken Turtle: an unterminated triple / garbage that no parser accepts.
    write(
        root,
        "slices/grpA/sliceA/module.ttl",
        "@prefix vocab: <https://example.org/vocab/> .\n\
         vocab:Broken a vocab:Class  <<< not turtle at all ;;;\n",
    );

    let catalog = SliceCatalog::discover(root, test_vocab()).unwrap();
    let result = OwnershipAnalyzer::new(&catalog).analyze();
    assert!(
        result.is_err(),
        "a malformed ownership artifact must hard-fail analysis, not be silently skipped"
    );
}
