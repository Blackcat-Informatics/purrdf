// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Acceptance tests for SliceCatalog: path-independence, lossless round-trip,
//! and recoverability.

use purrdf_slice::SliceVocab;
use purrdf_slice::artifact::ArtifactRole;
use purrdf_slice::catalog::SliceCatalog;

/// Pure fixtures use a caller-supplied example.org vocabulary.
fn test_vocab() -> SliceVocab {
    SliceVocab::for_namespace("https://example.org/vocab/")
}

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-slice")
}

// ── (a) Path-independence ─────────────────────────────────────────────────────

/// Copies the fixture slice into a tempdir, builds both catalogs, and asserts
/// that every artifact has the same raw_digest regardless of filesystem location.
#[test]
fn path_independence() {
    let src = fixture_dir();
    let tmp = tempfile::tempdir().expect("tempdir");
    let dst = tmp.path().join("test-slice");
    copy_dir_all(&src, &dst).expect("copy fixture");

    let rec_src = SliceCatalog::from_slice_dir(&src, &test_vocab()).expect("load from src");
    let rec_dst = SliceCatalog::from_slice_dir(&dst, &test_vocab()).expect("load from dst");

    assert_eq!(
        rec_src.artifacts.len(),
        rec_dst.artifacts.len(),
        "artifact count must match"
    );

    let mut src_digests: Vec<(&str, &str)> = rec_src
        .artifacts
        .iter()
        .map(|a| (a.logical_path.as_str(), a.raw_digest.as_str()))
        .collect();
    let mut dst_digests: Vec<(&str, &str)> = rec_dst
        .artifacts
        .iter()
        .map(|a| (a.logical_path.as_str(), a.raw_digest.as_str()))
        .collect();

    src_digests.sort_unstable();
    dst_digests.sort_unstable();

    assert_eq!(
        src_digests, dst_digests,
        "raw digests must be path-independent"
    );
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

// ── (b) Lossless round-trip ───────────────────────────────────────────────────

/// After loading the test-slice, verify that the manifest fields are parsed
/// correctly — IRI, title, identifier, and consumer text.
#[test]
fn lossless_round_trip() {
    let dir = fixture_dir();
    let rec = SliceCatalog::from_slice_dir(&dir, &test_vocab()).expect("load slice");

    // Structural fields from manifest.ttl.
    assert_eq!(
        rec.manifest.slice_iri,
        "https://example.org/test/slice/test"
    );
    assert_eq!(rec.manifest.title.as_deref(), Some("Test Slice"));
    assert_eq!(rec.manifest.identifier.as_deref(), Some("10.99999/test"));
    assert!(
        rec.manifest.consumers.iter().any(|c| c.contains("testing")),
        "consumers should contain 'testing'"
    );

    // The manifest graph must be non-empty (lossless: every triple survived).
    assert!(
        rec.manifest_graph.quad_count() > 0,
        "manifest_graph must contain quads"
    );

    // The unknown custom property triple must survive in the IR graph.
    // We verify it's there by checking the quad count includes it.
    // manifest.ttl has: a, label, title, creator, identifier, tier, consumer, customProp = 8 triples.
    assert!(
        rec.manifest_graph.quad_count() >= 8,
        "manifest_graph should contain at least 8 triples (got {})",
        rec.manifest_graph.quad_count()
    );
}

// ── (c) Recoverability ────────────────────────────────────────────────────────

/// For every artifact in the test slice, assert that find_artifact and
/// find_by_digest both return the same record, and that content is non-empty.
#[test]
fn recoverability() {
    let dir = fixture_dir();
    let rec = SliceCatalog::from_slice_dir(&dir, &test_vocab()).expect("load slice");

    assert!(
        !rec.artifacts.is_empty(),
        "test-slice must have at least one artifact"
    );

    for artifact in &rec.artifacts {
        // find_artifact must locate it by role+path.
        let found = rec.find_artifact(&artifact.role, &artifact.logical_path);
        assert!(
            found.is_some(),
            "find_artifact({:?}, {:?}) returned None",
            artifact.role,
            artifact.logical_path
        );
        assert_eq!(
            found.unwrap().raw_digest,
            artifact.raw_digest,
            "find_artifact returned wrong artifact"
        );

        // find_by_digest must also find it.
        let found2 = rec.find_by_digest(&artifact.raw_digest);
        assert!(
            found2.is_some(),
            "find_by_digest({:?}) returned None",
            artifact.raw_digest
        );

        // Content must be non-empty.
        assert!(
            !artifact.content.is_empty(),
            "artifact {:?} has empty content",
            artifact.logical_path
        );
    }
}

// ── (d) Role classification ───────────────────────────────────────────────────

/// Verify that the three fixture files are classified with the correct roles.
#[test]
fn role_classification() {
    let dir = fixture_dir();
    let rec = SliceCatalog::from_slice_dir(&dir, &test_vocab()).expect("load slice");

    let has_manifest = rec
        .artifacts
        .iter()
        .any(|a| matches!(a.role, ArtifactRole::Manifest));
    let has_module = rec
        .artifacts
        .iter()
        .any(|a| matches!(a.role, ArtifactRole::Module));
    let has_docs = rec
        .artifacts
        .iter()
        .any(|a| matches!(a.role, ArtifactRole::Documentation));

    assert!(has_manifest, "manifest.ttl must be classified as Manifest");
    assert!(has_module, "module.ttl must be classified as Module");
    assert!(has_docs, "docs.md must be classified as Documentation");
}

// ── (e) Semantic-digest blank-node determinism ────────────────────────────────

/// A module that uses blank nodes (e.g. an OWL restriction) must produce a STABLE
/// `semantic_digest` across repeated loads. Oxigraph assigns blank-node labels
/// non-deterministically at parse time, so the digest must canonicalize blank
/// nodes ( §12 — the semantic Merkle key must be deterministic). A
/// comment-only edit must NOT change the semantic digest.
#[test]
fn semantic_digest_blank_nodes_are_deterministic() {
    const NS: &str = "https://example.org/vocab/";

    fn write_slice(dir: &std::path::Path, module_comment: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let manifest = format!(
            r#"@prefix vocab: <{NS}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .

<{NS}slice/bn> a vocab:Slice ;
    rdfs:label "bn"@x-purrdf-english ;
    dcterms:title "bn"@x-purrdf-english ;
    dcterms:creator "Test" ;
    vocab:sliceTier vocab:tierCore ;
    vocab:sliceConsumer "test"@x-purrdf-english .
"#
        );
        std::fs::write(dir.join("manifest.ttl"), manifest).unwrap();
        // An OWL restriction (blank node) plus a second blank node, so the
        // parser is forced to mint multiple blank-node labels.
        let module = format!(
            r"{module_comment}@prefix vocab: <{NS}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .

<{NS}Bn> a owl:Class ;
    rdfs:isDefinedBy <{NS}slice/bn> ;
    rdfs:subClassOf [ a owl:Restriction ;
        owl:onProperty <{NS}hasThing> ;
        owl:someValuesFrom [ a owl:Class ] ] .
"
        );
        std::fs::write(dir.join("module.ttl"), module).unwrap();
    }

    fn module_semantic_digest(dir: &std::path::Path) -> String {
        let rec = SliceCatalog::from_slice_dir(dir, &test_vocab()).expect("load slice");
        rec.artifacts
            .iter()
            .find(|a| matches!(a.role, ArtifactRole::Module))
            .and_then(|a| a.semantic_digest.clone())
            .expect("module must carry a semantic digest")
    }

    let t1 = tempfile::tempdir().unwrap();
    let d1 = t1.path().join("bn");
    write_slice(&d1, "# comment A\n");

    // Repeated load of the SAME directory → identical digest (no parse drift).
    let a = module_semantic_digest(&d1);
    let b = module_semantic_digest(&d1);
    assert_eq!(
        a, b,
        "blank-node semantic digest must be stable across repeated loads"
    );

    // A comment-only edit → identical semantic digest (canonical RDF unchanged).
    let t2 = tempfile::tempdir().unwrap();
    let d2 = t2.path().join("bn");
    write_slice(&d2, "# a totally different comment\n# extra line\n");
    let c = module_semantic_digest(&d2);
    assert_eq!(
        a, c,
        "comment-only edit must not change the blank-node semantic digest"
    );
}
