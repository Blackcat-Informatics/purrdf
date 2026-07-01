// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Acceptance tests for the phase-specific Merkle cache + SCC/profile
//! composition (RFC #820 §12 / §8, child S6a). All fixtures are hermetic
//! (`tempfile`); no repository state is read.

use std::fmt::Write as _;
use std::path::Path;

use tempfile::TempDir;

use crate::cache::{
    dependency_closure, link_units, product_unit, source_unit_key, Phase, ToolchainContext,
};
use crate::catalog::SliceCatalog;
use crate::ownership::OwnershipAnalyzer;

const PURRDF: &str = "https://blackcatinformatics.ca/purrdf/";

fn toolchain() -> ToolchainContext {
    ToolchainContext::new("purrdf-logic v1", "native")
}

/// Write a minimal slice directory under `parent/<dirname>/`. The manifest
/// declares `slice_iri`, tier core, and (optionally) `purrdf:sliceDependsOn`
/// targets. The module defines `term` via `rdfs:isDefinedBy slice_iri` and
/// references each `dep_term` (so the dependency analyzer derives an edge).
fn write_slice(
    parent: &Path,
    dirname: &str,
    slice_iri: &str,
    term: &str,
    deps: &[(&str, &str)], // (dep_slice_iri, dep_term_iri)
    module_comment: &str,
) {
    let dir = parent.join(dirname);
    std::fs::create_dir_all(&dir).unwrap();

    let depends_on: String = deps.iter().fold(String::new(), |mut out, (dep_iri, _)| {
        let _ = writeln!(out, "    purrdf:sliceDependsOn <{dep_iri}> ;");
        out
    });

    // Content is keyed on the slice IRI, NEVER the directory name, so a copy at a
    // different path is byte-identical (the rename-invariance fixture relies on
    // this).
    let manifest = format!(
        r#"@prefix purrdf: <{PURRDF}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .

<{slice_iri}> a purrdf:Slice ;
    rdfs:label "slice label"@x-purrdf-english ;
    dcterms:title "Slice title"@x-purrdf-english ;
    dcterms:creator "Test Author" ;
    purrdf:sliceTier purrdf:tierCore ;
{depends_on}    purrdf:sliceConsumer "test"@x-purrdf-english .
"#
    );
    std::fs::write(dir.join("manifest.ttl"), manifest).unwrap();

    // Module: define `term`, reference each dep term. A leading comment lets the
    // raw bytes vary while the canonical RDF stays identical.
    let refs: String =
        deps.iter()
            .enumerate()
            .fold(String::new(), |mut out, (i, (_, dep_term))| {
                let _ = writeln!(out, "<{term}> purrdf:refP{i} <{dep_term}> .");
                out
            });
    let module = format!(
        r#"{module_comment}@prefix purrdf: <{PURRDF}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .

<{term}> a owl:Class ;
    rdfs:isDefinedBy <{slice_iri}> ;
    rdfs:label "term"@x-purrdf-english .
{refs}"#
    );
    std::fs::write(dir.join("module.ttl"), module).unwrap();
}

fn discover(root: &Path) -> (SliceCatalog, Vec<crate::ownership::DependencyEdge>) {
    let catalog = SliceCatalog::discover(root).unwrap();
    let report = OwnershipAnalyzer::new(&catalog).analyze().unwrap();
    (catalog, report.edges)
}

/// Acceptance 1 — **rename invariance**: building keys for a fixture slice, then
/// for a byte-identical copy at a *different directory path*, yields identical
/// per-phase keys. Moving the slice's directory changes NO cache key.
#[test]
fn rename_invariance_all_phases() {
    let a_iri = format!("{PURRDF}slice/alpha");
    let a_term = format!("{PURRDF}Alpha");

    // Layout 1: slices/core/alpha
    let t1 = TempDir::new().unwrap();
    let core1 = t1.path().join("slices").join("core");
    write_slice(&core1, "alpha", &a_iri, &a_term, &[], "# comment v1\n");

    // Layout 2: a totally different group path — slices/zztop/renamed
    let t2 = TempDir::new().unwrap();
    let zz = t2.path().join("slices").join("zztop");
    write_slice(&zz, "renamed", &a_iri, &a_term, &[], "# comment v1\n");

    let (cat1, edges1) = discover(t1.path());
    let (cat2, edges2) = discover(t2.path());
    let tc = toolchain();

    for phase in [
        Phase::Parse,
        Phase::Syntax,
        Phase::Shacl,
        Phase::Reason,
        Phase::Bundle,
    ] {
        let k1 = source_unit_key(phase, &cat1, &edges1, &a_iri, &tc).unwrap();
        let k2 = source_unit_key(phase, &cat2, &edges2, &a_iri, &tc).unwrap();
        assert_eq!(
            k1.root, k2.root,
            "phase {phase:?} key changed across a directory rename"
        );
    }
}

/// Acceptance 2 — **comment-only invariance of the reasoning key**: changing a
/// comment in a module (raw bytes differ, canonical RDF identical) leaves the
/// REASONING-phase key unchanged, while the SYNTAX (byte-sensitive) key changes.
#[test]
fn comment_only_invariance_reasoning_key() {
    let a_iri = format!("{PURRDF}slice/alpha");
    let a_term = format!("{PURRDF}Alpha");

    let t1 = TempDir::new().unwrap();
    let core1 = t1.path().join("slices").join("core");
    write_slice(
        &core1,
        "alpha",
        &a_iri,
        &a_term,
        &[],
        "# original comment\n",
    );

    let t2 = TempDir::new().unwrap();
    let core2 = t2.path().join("slices").join("core");
    write_slice(
        &core2,
        "alpha",
        &a_iri,
        &a_term,
        &[],
        "# a COMPLETELY different comment line\n# with an extra line too\n",
    );

    let (cat1, edges1) = discover(t1.path());
    let (cat2, edges2) = discover(t2.path());
    let tc = toolchain();

    let reason_before = source_unit_key(Phase::Reason, &cat1, &edges1, &a_iri, &tc).unwrap();
    let reason_after = source_unit_key(Phase::Reason, &cat2, &edges2, &a_iri, &tc).unwrap();
    assert_eq!(
        reason_before.root, reason_after.root,
        "reasoning key changed on a comment-only edit (canonical RDF identical)"
    );

    let syntax_before = source_unit_key(Phase::Syntax, &cat1, &edges1, &a_iri, &tc).unwrap();
    let syntax_after = source_unit_key(Phase::Syntax, &cat2, &edges2, &a_iri, &tc).unwrap();
    assert_ne!(
        syntax_before.root, syntax_after.root,
        "syntax (byte-sensitive) key did NOT change on a raw-bytes edit"
    );
}

/// Acceptance 3 — **SCC grouping**: A↔B mutually dependent and C→A yields one
/// link unit {A,B} and a singleton {C}; A and B remain individually nameable.
#[test]
fn scc_grouping_cycle_and_singleton() {
    let a_iri = format!("{PURRDF}slice/aaa");
    let b_iri = format!("{PURRDF}slice/bbb");
    let c_iri = format!("{PURRDF}slice/ccc");
    let a_term = format!("{PURRDF}Aaa");
    let b_term = format!("{PURRDF}Bbb");
    let c_term = format!("{PURRDF}Ccc");

    let t = TempDir::new().unwrap();
    let core = t.path().join("slices").join("core");
    // A depends on B, B depends on A → cycle. C depends on A.
    write_slice(&core, "aaa", &a_iri, &a_term, &[(&b_iri, &b_term)], "# a\n");
    write_slice(&core, "bbb", &b_iri, &b_term, &[(&a_iri, &a_term)], "# b\n");
    write_slice(&core, "ccc", &c_iri, &c_term, &[(&a_iri, &a_term)], "# c\n");

    let (catalog, edges) = discover(t.path());
    let units = link_units(&catalog, &edges);

    // Find the cyclic unit and the singleton.
    let cycle = units
        .iter()
        .find(|u| u.is_cycle())
        .expect("expected one SCC cycle");
    assert_eq!(cycle.members.len(), 2);
    assert!(cycle.contains(&a_iri) && cycle.contains(&b_iri));

    let singleton = units
        .iter()
        .find(|u| u.members == vec![c_iri.clone()])
        .expect("expected a singleton link unit {C}");
    assert!(singleton.contains(&c_iri));
    assert!(!singleton.is_cycle());

    // A and B remain individually nameable within the link unit.
    assert!(cycle.contains(&a_iri));
    assert!(cycle.contains(&b_iri));
}

/// Acceptance 4 — **profile closure**: a product unit for a slice with deps
/// yields the dependency-closed set (transitive closure).
#[test]
fn profile_closure_transitive() {
    let a_iri = format!("{PURRDF}slice/aaa");
    let b_iri = format!("{PURRDF}slice/bbb");
    let c_iri = format!("{PURRDF}slice/ccc");
    let a_term = format!("{PURRDF}Aaa");
    let b_term = format!("{PURRDF}Bbb");
    let c_term = format!("{PURRDF}Ccc");

    let t = TempDir::new().unwrap();
    let core = t.path().join("slices").join("core");
    // A → B → C (a transitive chain).
    write_slice(&core, "aaa", &a_iri, &a_term, &[(&b_iri, &b_term)], "# a\n");
    write_slice(&core, "bbb", &b_iri, &b_term, &[(&c_iri, &c_term)], "# b\n");
    write_slice(&core, "ccc", &c_iri, &c_term, &[], "# c\n");

    let (catalog, edges) = discover(t.path());

    let closure = dependency_closure(&catalog, &edges, std::slice::from_ref(&a_iri));
    assert!(closure.contains(&a_iri));
    assert!(closure.contains(&b_iri));
    assert!(
        closure.contains(&c_iri),
        "transitive dep C must be in closure"
    );
    assert_eq!(closure.len(), 3);

    let product = product_unit(&catalog, &edges, std::slice::from_ref(&a_iri));
    assert_eq!(product.seeds, vec![a_iri]);
    assert_eq!(product.closure.len(), 3);

    // C alone closes to just {C}.
    let c_closure = dependency_closure(&catalog, &edges, std::slice::from_ref(&c_iri));
    assert_eq!(c_closure, vec![c_iri]);
}

/// Extra guard: a dependency-output digest change DOES invalidate the
/// dependent's reasoning key (Merkle composition), while a comment-only change
/// to that dependency does not.
#[test]
fn dependency_change_invalidates_dependent_reasoning_key() {
    let a_iri = format!("{PURRDF}slice/aaa");
    let b_iri = format!("{PURRDF}slice/bbb");
    let a_term = format!("{PURRDF}Aaa");
    let b_term = format!("{PURRDF}Bbb");
    let b_term2 = format!("{PURRDF}BbbExtra");

    // Baseline: A depends on B; B defines one term.
    let t1 = TempDir::new().unwrap();
    let core1 = t1.path().join("slices").join("core");
    write_slice(
        &core1,
        "aaa",
        &a_iri,
        &a_term,
        &[(&b_iri, &b_term)],
        "# a\n",
    );
    write_slice(&core1, "bbb", &b_iri, &b_term, &[], "# b\n");

    // Variant: B's module gains a real (canonical) triple defining an extra term.
    let t2 = TempDir::new().unwrap();
    let core2 = t2.path().join("slices").join("core");
    write_slice(
        &core2,
        "aaa",
        &a_iri,
        &a_term,
        &[(&b_iri, &b_term)],
        "# a\n",
    );
    {
        let dir = core2.join("bbb");
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = format!(
            r#"@prefix purrdf: <{PURRDF}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .

<{b_iri}> a purrdf:Slice ;
    rdfs:label "slice bbb"@x-purrdf-english ;
    dcterms:title "Slice bbb"@x-purrdf-english ;
    dcterms:creator "Test Author" ;
    purrdf:sliceTier purrdf:tierCore ;
    purrdf:sliceConsumer "test"@x-purrdf-english .
"#
        );
        std::fs::write(dir.join("manifest.ttl"), manifest).unwrap();
        let module = format!(
            r#"# b
@prefix purrdf: <{PURRDF}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .

<{b_term}> a owl:Class ;
    rdfs:isDefinedBy <{b_iri}> ;
    rdfs:label "term"@x-purrdf-english .
<{b_term2}> a owl:Class ;
    rdfs:isDefinedBy <{b_iri}> ;
    rdfs:label "extra"@x-purrdf-english .
"#
        );
        std::fs::write(dir.join("module.ttl"), module).unwrap();
    }

    let (cat1, edges1) = discover(t1.path());
    let (cat2, edges2) = discover(t2.path());
    let tc = toolchain();

    let a_before = source_unit_key(Phase::Reason, &cat1, &edges1, &a_iri, &tc).unwrap();
    let a_after = source_unit_key(Phase::Reason, &cat2, &edges2, &a_iri, &tc).unwrap();
    assert_ne!(
        a_before.root, a_after.root,
        "A's reasoning key must change when its dependency B changes semantically"
    );
}

/// Write a single slice with a fully explicit manifest body (so the manifest's
/// canonical RDF and comments can be controlled independently of the module).
/// The module always defines `term` via `rdfs:isDefinedBy slice_iri`.
fn write_slice_explicit_manifest(
    parent: &Path,
    dirname: &str,
    slice_iri: &str,
    term: &str,
    manifest_body: &str,
) {
    let dir = parent.join(dirname);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("manifest.ttl"), manifest_body).unwrap();
    let module = format!(
        r#"@prefix purrdf: <{PURRDF}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .

<{term}> a owl:Class ;
    rdfs:isDefinedBy <{slice_iri}> ;
    rdfs:label "term"@x-purrdf-english .
"#
    );
    std::fs::write(dir.join("module.ttl"), module).unwrap();
}

/// HIGH-6 — **manifest participates in the semantic (Reason/Shacl) key**: a
/// manifest comment-only edit leaves the Reason/Shacl key UNCHANGED (manifest is
/// folded by its *semantic* digest), while a real semantic manifest change (a new
/// `purrdf:sliceDependsOn`) DOES change the Reason/Shacl key.
#[test]
fn manifest_folds_into_semantic_phases() {
    let a_iri = format!("{PURRDF}slice/alpha");
    let a_term = format!("{PURRDF}Alpha");
    let dep_iri = format!("{PURRDF}slice/dep");

    let base_manifest = format!(
        r#"@prefix purrdf: <{PURRDF}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .

<{a_iri}> a purrdf:Slice ;
    rdfs:label "slice label"@x-purrdf-english ;
    dcterms:title "Slice title"@x-purrdf-english ;
    dcterms:creator "Test Author" ;
    purrdf:sliceTier purrdf:tierCore ;
    purrdf:sliceConsumer "test"@x-purrdf-english .
"#
    );

    // Variant 1: identical canonical RDF, different comment only.
    let comment_only_manifest = format!("# a different leading comment\n{base_manifest}");

    // Variant 2: a real semantic change — add a purrdf:sliceDependsOn triple.
    let semantic_change_manifest = format!(
        r#"@prefix purrdf: <{PURRDF}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .

<{a_iri}> a purrdf:Slice ;
    rdfs:label "slice label"@x-purrdf-english ;
    dcterms:title "Slice title"@x-purrdf-english ;
    dcterms:creator "Test Author" ;
    purrdf:sliceTier purrdf:tierCore ;
    purrdf:sliceDependsOn <{dep_iri}> ;
    purrdf:sliceConsumer "test"@x-purrdf-english .
"#
    );

    let t_base = TempDir::new().unwrap();
    write_slice_explicit_manifest(
        &t_base.path().join("slices").join("core"),
        "alpha",
        &a_iri,
        &a_term,
        &base_manifest,
    );
    let t_comment = TempDir::new().unwrap();
    write_slice_explicit_manifest(
        &t_comment.path().join("slices").join("core"),
        "alpha",
        &a_iri,
        &a_term,
        &comment_only_manifest,
    );
    let t_semantic = TempDir::new().unwrap();
    write_slice_explicit_manifest(
        &t_semantic.path().join("slices").join("core"),
        "alpha",
        &a_iri,
        &a_term,
        &semantic_change_manifest,
    );

    let (cat_base, edges_base) = discover(t_base.path());
    let (cat_comment, edges_comment) = discover(t_comment.path());
    let (cat_semantic, edges_semantic) = discover(t_semantic.path());
    let tc = toolchain();

    for phase in [Phase::Reason, Phase::Shacl] {
        let base = source_unit_key(phase, &cat_base, &edges_base, &a_iri, &tc).unwrap();
        let comment = source_unit_key(phase, &cat_comment, &edges_comment, &a_iri, &tc).unwrap();
        let semantic = source_unit_key(phase, &cat_semantic, &edges_semantic, &a_iri, &tc).unwrap();

        assert_eq!(
            base.root, comment.root,
            "phase {phase:?}: a comment-only manifest edit must NOT change the key \
             (manifest folds by its semantic digest)"
        );
        assert_ne!(
            base.root, semantic.root,
            "phase {phase:?}: a semantic manifest change (new sliceDependsOn) MUST \
             change the key (manifest is folded under semantic phases)"
        );
    }

    // The byte-sensitive Syntax phase already folded the manifest; a comment-only
    // manifest edit changes ITS key (manifest is byte-sensitive there).
    let syntax_base = source_unit_key(Phase::Syntax, &cat_base, &edges_base, &a_iri, &tc).unwrap();
    let syntax_comment =
        source_unit_key(Phase::Syntax, &cat_comment, &edges_comment, &a_iri, &tc).unwrap();
    assert_ne!(
        syntax_base.root, syntax_comment.root,
        "Syntax (byte-sensitive) key MUST change on a comment-only manifest edit"
    );
}
