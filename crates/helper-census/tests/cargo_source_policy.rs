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
    let shared = root.join("shared");
    let outside = directory.path().join("outside");
    for package in [&root, &consumer, &shared, &outside] {
        fs::create_dir_all(package).expect("create fixture package directory");
        fs::write(package.join("Cargo.toml"), "").expect("create local manifest");
    }
    let cargo_home = directory.path().join("cargo-home");
    fs::create_dir(&cargo_home).expect("create private home");
    let root_graph = directory.path().join("root.json");
    let consumer_graph = directory.path().join("consumer.json");
    graph(&root_graph, &root, &shared);
    graph(&consumer_graph, &consumer, &shared);
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

fn local_package(path: &Path, name: &str) -> Object {
    let path = path.to_str().expect("UTF-8 test path");
    Object::new()
        .with("id", format!("path+file://{path}#{name}@1.0.0"))
        .with("name", name)
        .with("version", "1.0.0")
        .with("source", Value::Null)
        .with("manifest_path", format!("{path}/Cargo.toml"))
}

fn graph(output: &Path, workspace: &Path, dependency: &Path) {
    write_graph(
        output,
        workspace,
        local_package(dependency, "fixture-dependency"),
        Value::from(dependency.to_str().expect("UTF-8 test path")),
    );
}

fn write_graph(output: &Path, workspace: &Path, dependency: Object, path: Value) {
    let dependency_id = dependency.get("id").expect("dependency identity").clone();
    let dependency_name = dependency.get("name").expect("dependency name").clone();
    let source = dependency.get("source").expect("dependency source").clone();
    let member = local_package(workspace, "policy-fixture")
        .with("publish", Vec::<Value>::new())
        .with(
            "dependencies",
            vec![Value::from(
                Object::new()
                    .with("name", dependency_name.clone())
                    .with("source", source)
                    .with("kind", Value::Null)
                    .with("path", path),
            )],
        );
    let member_id = member
        .get("id")
        .expect("workspace package identity")
        .clone();
    let edge_name = dependency_name
        .as_str()
        .expect("Cargo package name")
        .replace('-', "_");
    let edge = Object::new()
        .with("name", edge_name)
        .with("pkg", dependency_id.clone())
        .with(
            "dep_kinds",
            vec![Value::from(
                Object::new()
                    .with("kind", Value::Null)
                    .with("target", Value::Null),
            )],
        );
    let nodes = vec![
        Value::from(
            Object::new()
                .with("id", member_id.clone())
                .with("dependencies", vec![dependency_id.clone()])
                .with("deps", vec![Value::from(edge)]),
        ),
        Value::from(
            Object::new()
                .with("id", dependency_id)
                .with("dependencies", Vec::<Value>::new())
                .with("deps", Vec::<Value>::new()),
        ),
    ];
    let value = Value::from(
        Object::new()
            .with(
                "workspace_root",
                workspace.to_str().expect("UTF-8 test path"),
            )
            .with("workspace_members", vec![member_id.clone()])
            .with(
                "resolve",
                Object::new().with("root", member_id).with("nodes", nodes),
            )
            .with(
                "packages",
                vec![
                    Value::from(member),
                    Value::from(dependency.with("dependencies", Vec::<Value>::new())),
                ],
            ),
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
    for path in [&root, &consumer, &package, &replacement, &external] {
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
                    "id",
                    "registry+https://github.com/rust-lang/crates.io-index#example@1.0.0",
                )
                .with("name", "example")
                .with("version", "1.0.0")
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
        };
        write_graph(&root_graph, &root, graph_package(), Value::Null);
        write_graph(&consumer_graph, &consumer, graph_package(), Value::Null);
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
