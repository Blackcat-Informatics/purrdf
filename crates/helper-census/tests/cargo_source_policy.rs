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
    let root_graph = directory.path().join("root.json");
    let consumer_graph = directory.path().join("consumer.json");
    graph(&root_graph, &root, &root);
    graph(&consumer_graph, &consumer, &root);
    let mut command = checker(&root);
    command.arg(&root_graph).arg(&consumer_graph);
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
fn cargo_config_includes_are_contained_and_admit_optional_absence() {
    let directory = purrdf_testkit::temp_dir!().expect("allocate config fixture");
    let root = directory.path().join("snapshot");
    let config = root.join(".cargo");
    fs::create_dir_all(&config).expect("create config directory");
    let top = config.join("config.toml");
    let intermediate = config.join("inside.toml");
    fs::write(
        &top,
        r#"include = ["inside.toml", { path = "optional.toml", optional = true }]"#,
    )
    .expect("write staged config");
    fs::write(&intermediate, "[net]\nretry = 3").expect("write in-tree include");
    let admitted = execute(&mut checker(&root));
    assert!(
        admitted.status.success(),
        "{}",
        String::from_utf8_lossy(&admitted.stderr)
    );
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
