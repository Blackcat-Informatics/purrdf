// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Project the shared workload tooling onto a selected production crate graph.

use std::path::{Path, PathBuf};

fn checked_path(path: &Path) -> &str {
    let path = path.to_str().expect("UTF-8 production crate path");
    assert!(
        path.chars()
            .all(|c| !c.is_control() && c != '"' && c != '\\'),
        "production crate path contains unsupported TOML string characters"
    );
    path
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(args.len(), 3, "expected SOURCE TOOLING PROJECT");
    let source = PathBuf::from(&args[0]);
    let tooling = PathBuf::from(&args[1]);
    let project = PathBuf::from(&args[2]);
    let root_manifest = std::fs::read_to_string(tooling.join("Cargo.toml"))
        .expect("read tooling workspace manifest");
    let (_, workspace) = root_manifest
        .split_once("[workspace.package]")
        .expect("tooling workspace must declare package metadata");
    let workspace = workspace
        .split_once("[profile.release]")
        .map_or(workspace, |(tables, _)| tables);
    let hash_path = source.join("crates/hash");
    let hash_path = checked_path(&hash_path);
    let hash_home = "path = \"crates/hash\"";
    assert_eq!(workspace.matches(hash_home).count(), 1, "one hash home");
    let workspace = workspace.replace(hash_home, &format!("path = \"{hash_path}\""));
    let mut manifest = String::from(
        "[package]\nname=\"hash-workloads\"\nversion=\"0.0.0\"\nedition=\"2024\"\npublish=false\n\
         [workspace]\nresolver=\"2\"\nmembers=[\"crates/testkit\"]\n[dependencies]\n",
    );
    for (name, directory) in [
        ("purrdf-core", "rdf-core"),
        ("purrdf-rdf", "rdf"),
        ("purrdf-gts", "gts"),
        ("purrdf-sparql-eval", "sparql-eval"),
        ("purrdf-alloc-probe", "alloc-probe"),
    ] {
        let path = source.join("crates").join(directory);
        let path = checked_path(&path);
        manifest.push_str(&format!("{name} = {{ path = \"{path}\" }}\n"));
    }
    manifest.push_str(
        "purrdf-testkit = { path = \"crates/testkit\" }\n\
         [[bin]]\nname=\"timing\"\npath=\"src/timing.rs\"\n\
         [[bin]]\nname=\"allocations\"\npath=\"src/allocations.rs\"\n\
         [workspace.package]\n",
    );
    manifest.push_str(&workspace);
    std::fs::write(project.join("Cargo.toml"), manifest).expect("write workload manifest");
}
