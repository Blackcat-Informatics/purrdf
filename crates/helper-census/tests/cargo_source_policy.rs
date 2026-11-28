// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exercise the checker used by the locked Cargo source-policy recipe.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use purrdf_lex::json::{self, Object, Value};

fn checker(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_helper-census"));
    command
        .arg("--cargo-source-policy")
        .arg(root)
        .arg("consumer");
    command
}

fn execute(command: &mut Command) -> Output {
    command.output().expect("run built helper-census")
}

#[test]
fn local_cargo_graph_paths_cannot_resolve_outside_the_checkout() {
    let directory = purrdf_testkit::temp_dir!().expect("allocate graph fixture");
    let root = directory.path().join("snapshot");
    let consumer = root.join("consumer");
    fs::create_dir_all(&consumer).expect("create consumer");
    for workspace in [&root, &consumer] {
        fs::write(workspace.join("Cargo.toml"), "").expect("create local manifest");
    }
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).expect("create out-of-root fixture dependency");
    let cargo_home = directory.path().join("cargo-home");
    fs::create_dir(&cargo_home).expect("create private home");
    let root_graph = directory.path().join("root.json");
    let consumer_graph = directory.path().join("consumer.json");
    graph(&root_graph, &root, &root);
    graph(&consumer_graph, &consumer, &root);
    let mut command = checker(&root);
    command.args([&cargo_home, &root_graph, &consumer_graph]);
    let inside = execute(&mut command);
    assert!(
        inside.status.success(),
        "{}",
        String::from_utf8_lossy(&inside.stderr)
    );
    graph(&consumer_graph, &consumer, &outside);
    let refused = execute(&mut command);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("Cargo metadata local dependency path is outside the policy root")
    );
}

fn graph(output: &Path, workspace: &Path, dependency: &Path) {
    let manifest = workspace.join("Cargo.toml");
    let package = Object::new()
        .with("source", Value::Null)
        .with("manifest_path", manifest.to_str().expect("UTF-8 test path"))
        .with(
            "dependencies",
            vec![Value::from(
                Object::new().with("path", dependency.to_str().expect("UTF-8 test path")),
            )],
        );
    write_graph(output, workspace, package);
}

fn write_graph(output: &Path, workspace: &Path, package: Object) {
    let value = Value::from(
        Object::new()
            .with(
                "workspace_root",
                workspace.to_str().expect("UTF-8 test path"),
            )
            .with("resolve", Object::new().with("nodes", Vec::<Value>::new()))
            .with("packages", vec![Value::from(package)]),
    );
    fs::write(output, json::write_compact(&value)).expect("write synthetic locked metadata");
}

#[test]
fn registry_manifests_must_come_from_the_private_registry() {
    let directory = purrdf_testkit::temp_dir!().expect("allocate registry fixture");
    let root = directory.path().join("snapshot");
    let consumer = root.join("consumer");
    let cargo_home = directory.path().join("cargo-home");
    let package = cargo_home.join("registry/src/index/example-1.0.0");
    let replacement = root.join("vendor/example-1.0.0");
    let external = directory.path().join("vendor/example-1.0.0");
    for path in [&consumer, &package, &replacement, &external] {
        fs::create_dir_all(path).expect("create fixture directory");
        fs::write(path.join("Cargo.toml"), "").expect("create fixture manifest");
    }
    let root_graph = directory.path().join("root.json");
    let consumer_graph = directory.path().join("consumer.json");
    for (manifest_directory, admitted) in
        [(&package, true), (&replacement, false), (&external, false)]
    {
        let graph_package = || {
            Object::new()
                .with(
                    "source",
                    "registry+https://github.com/rust-lang/crates.io-index",
                )
                .with(
                    "manifest_path",
                    manifest_directory
                        .join("Cargo.toml")
                        .to_str()
                        .expect("UTF-8 test path"),
                )
                .with("dependencies", Vec::<Value>::new())
        };
        write_graph(&root_graph, &root, graph_package());
        write_graph(&consumer_graph, &consumer, graph_package());
        let output = execute(checker(&root).args([&cargo_home, &root_graph, &consumer_graph]));
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.success(), admitted, "{diagnostic}");
        if !admitted {
            assert!(
                diagnostic.contains(
                    "Cargo metadata registry package manifest is outside the policy root"
                )
            );
        }
    }
}

#[test]
fn cargo_config_includes_are_contained_and_admit_optional_absence() {
    let directory = purrdf_testkit::temp_dir!().expect("allocate config fixture");
    let root = directory.path().join("snapshot");
    let config = root.join(".cargo");
    fs::create_dir_all(&config).expect("create config directory");
    let top = config.join("config.toml");
    let intermediate = config.join("inside.toml");
    fs::write(&intermediate, "[net]\nretry = 3").expect("write in-tree include");
    for config in [
        r#"include = ["inside.toml", { path = "optional.toml", optional = true }]"#,
        "[[include]]\npath = \"inside.toml\"\n[[include]]\npath = \"optional.toml\"\noptional = true\n",
    ] {
        fs::write(&top, config).expect("write admitted include syntax");
        let admitted = execute(&mut checker(&root));
        assert!(
            admitted.status.success(),
            "{}",
            String::from_utf8_lossy(&admitted.stderr)
        );
    }
    fs::write(directory.path().join("outside.toml"), "[net]\nretry = 7")
        .expect("create external config");
    fs::write(&intermediate, r#"include = ["../../outside.toml"]"#)
        .expect("point transitive config outside");
    let refused = execute(&mut checker(&root));
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("Cargo config include is outside the policy root")
    );
}
