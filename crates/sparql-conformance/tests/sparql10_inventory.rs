// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native completeness and byte-freeze tripwires for the W3C data-r2 import.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// Observe the production guard, without adding assertions to its Python bridge.
fn loaded_receipt(script: &Path, root: &str) -> Output {
    Command::new("python3")
        .arg("-c")
        .arg("import runpy,sys; print(runpy.run_path(sys.argv[1])['GUARDED_ROOTS'][sys.argv[2]])")
        .arg(script)
        .arg(root)
        .output()
        .expect("read the production freeze registry")
}

#[test]
fn the_sparql10_payload_is_registered_with_its_exact_freeze_receipt() {
    const ROOT: &str = "crates/sparql-conformance/suite/w3c-sparql10";
    const RECEIPT: &str = "scripts/conformance-frozen/sparql-conformance-suite-w3c-sparql10.sha256";
    let workspace = purrdf_testkit::paths::workspace_root();
    // Observe the production guard's loaded registry. All expectations belong
    // to this native Rust test; this bridge only returns its configured value.
    let loaded = loaded_receipt(&workspace.join("scripts/check-corpus-frozen.py"), ROOT);
    assert!(
        loaded.status.success(),
        "{}",
        String::from_utf8_lossy(&loaded.stderr)
    );
    assert_eq!(
        String::from_utf8(loaded.stdout)
            .expect("receipt path is UTF-8")
            .trim(),
        RECEIPT
    );
    let receipt = fs::read_to_string(workspace.join(RECEIPT)).expect("frozen receipt exists");
    let mut files = BTreeSet::new();
    for line in receipt.lines() {
        let (digest, path) = line.split_once("  ").expect("SHA-256 receipt record");
        assert_eq!(digest.len(), 64);
        assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(files.insert(path), "duplicate frozen path {path}");
        assert!(workspace.join(ROOT).join(path).is_file(), "{path}");
    }
    assert_eq!(files.len(), 875, "every pinned upstream payload is frozen");
    assert!(files.contains("manifest-all.ttl"));
    assert!(files.contains("sort/extended-manifest.ttl"));
}

#[test]
fn a_missing_or_malformed_freeze_registry_is_refused() {
    let workspace = purrdf_testkit::paths::workspace_root();
    let fixture = purrdf_testkit::temp_dir!("sparql10-freeze-registry-").expect("fixture");
    let script = fixture.path().join("check-corpus-frozen.py");
    fs::copy(workspace.join("scripts/check-corpus-frozen.py"), &script).expect("copy guard");
    assert!(!loaded_receipt(&script, "fixture").status.success());
    let registry_dir = fixture.path().join("conformance-frozen");
    fs::create_dir(&registry_dir).expect("registry directory");
    let registry = registry_dir.join("roots.toml");
    fs::write(&registry, "[roots]\n").expect("empty registry");
    let empty = Command::new("python3")
        .arg(&script)
        .output()
        .expect("run the production freeze guard");
    assert!(
        !empty.status.success(),
        "an empty registry must not pass vacuously: {}",
        String::from_utf8_lossy(&empty.stdout)
    );
    // The valid neighbour observes an exact receipt from the same guard.
    fs::write(&registry, "[roots]\nfixture = \"receipt.sha256\"\n").expect("valid registry");
    let valid = loaded_receipt(&script, "fixture");
    assert!(valid.status.success());
    assert_eq!(valid.stdout, b"receipt.sha256\n");
    for malformed in [
        "[roots\n",
        "unrelated = \"receipt.sha256\"\n",
        "[roots]\nfixture = 1\n",
        "[roots]\nfixture = \"\"\n",
        "[roots]\n\"\" = \"receipt.sha256\"\n",
    ] {
        fs::write(&registry, malformed).expect("malformed registry");
        assert!(!loaded_receipt(&script, "fixture").status.success());
    }
}

#[test]
fn every_data_r2_leaf_and_the_root_closure_have_the_exact_pinned_inventory() {
    use purrdf_sparql_conformance::{manifest, paths};
    const GROUPS: &[(&str, usize)] = &[
        ("algebra", 14),
        ("ask", 4),
        ("basic", 27),
        ("bnode-coreference", 1),
        ("boolean-effective-value", 7),
        ("bound", 1),
        ("cast", 7),
        ("construct", 5),
        ("dataset", 12),
        ("distinct", 11),
        ("expr-builtin", 25),
        ("expr-equals", 15),
        ("expr-ops", 18),
        ("graph", 17),
        ("i18n", 5),
        ("open-world", 18),
        ("optional", 7),
        ("optional-filter", 5),
        ("reduced", 2),
        ("regex", 21),
        ("solution-seq", 13),
        ("sort", 14),
        ("syntax-sparql1", 81),
        ("syntax-sparql2", 53),
        ("syntax-sparql3", 51),
        ("syntax-sparql4", 12),
        ("syntax-sparql5", 2),
        ("triple-match", 4),
        ("type-promotion", 30),
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite/w3c-sparql10");
    let discovered = paths::suite_manifests(&root).expect("complete leaf discovery");
    let mut expected_leaves: BTreeSet<_> = GROUPS
        .iter()
        .map(|(group, _)| format!("{group}/manifest.ttl"))
        .collect();
    expected_leaves.insert("sort/extended-manifest.ttl".to_owned());
    assert_eq!(
        discovered
            .iter()
            .map(|leaf| leaf.relative.clone())
            .collect::<BTreeSet<_>>(),
        expected_leaves
    );
    assert_eq!(discovered.len(), 30);
    let mut declared = BTreeSet::new();
    for &(group, count) in GROUPS {
        let leaf = root.join(group).join("manifest.ttl");
        let cases = manifest::load(&leaf).unwrap_or_else(|error| panic!("{group}: {error}"));
        assert_eq!(cases.len(), count, "pinned {group} case count");
        for case in cases {
            assert_ne!(case.kind, manifest::TestKind::Unknown, "{}", case.iri);
            assert!(
                declared.insert(case.iri),
                "a case is executed by two leaves"
            );
        }
    }
    assert_eq!(declared.len(), 482);
    let closure = manifest::load(&root.join("manifest-all.ttl")).expect("pinned root closure");
    assert_eq!(closure.len(), 482);
    assert_eq!(
        closure
            .into_iter()
            .map(|case| case.iri)
            .collect::<BTreeSet<_>>(),
        declared
    );
    let extended =
        manifest::load(&root.join("sort/extended-manifest.ttl")).expect("extended sort leaf");
    assert_eq!(extended.len(), 1);
    assert!(extended[0].iri.ends_with("#dawg-sort-11"));
    assert!(declared.insert(extended[0].iri.clone()));
    assert_eq!(declared.len(), 483);
}
