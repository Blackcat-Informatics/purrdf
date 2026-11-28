// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Keep the GTS fuzz runtime and its native build inputs out of shipped graphs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use purrdf_lex::json::Value;

pub(crate) const WORKSPACE: &str = "crates/gts/fuzz";

const MEMBER: &str = "purrdf-gts-fuzz";
const GTS: &str = "purrdf-gts";
// Existing registry packages also used by non-fuzz graphs.
const SHARED_HOST_INPUTS: [&str; 4] = ["cfg-if", "getrandom", "libc", "r-efi"];

const REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";

struct Edge {
    parent: &'static str,
    package: &'static str,
    kind: Option<&'static str>,
}

const EDGES: [Edge; 6] = [
    Edge {
        parent: MEMBER,
        package: "libfuzzer-sys",
        kind: None,
    },
    Edge {
        parent: "libfuzzer-sys",
        package: "arbitrary",
        kind: None,
    },
    Edge {
        parent: "libfuzzer-sys",
        package: "cc",
        kind: Some("build"),
    },
    Edge {
        parent: "cc",
        package: "find-msvc-tools",
        kind: None,
    },
    Edge {
        parent: "cc",
        package: "jobserver",
        kind: None,
    },
    Edge {
        parent: "cc",
        package: "shlex",
        kind: None,
    },
];

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, String> {
    value
        .get(name)
        .ok_or_else(|| format!("Cargo metadata is missing {name}"))
}

fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    field(value, name)?
        .as_str()
        .ok_or_else(|| format!("Cargo metadata {name} must be a string"))
}

fn slice<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], String> {
    field(value, name)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("Cargo metadata {name} must be an array"))
}

fn matching_kind(dep: &Value, expected: Option<&str>) -> Result<bool, String> {
    let [kind] = slice(dep, "dep_kinds")? else {
        return Ok(false);
    };
    let actual = field(kind, "kind")?;
    let matches = match expected {
        Some(expected) => actual.as_str() == Some(expected),
        None => actual.is_null(),
    };
    Ok(matches && field(kind, "target")?.is_null())
}

/// Check an unfiltered locked resolution. Each isolated package has exactly
/// one incoming edge, so no alternate graph path can bypass the build edge.
pub(crate) fn validate(graph: &Value, root: &Path, workspace: &Path) -> Result<(), String> {
    if Path::new(string(graph, "workspace_root")?) != workspace {
        return Err("Cargo metadata has an unexpected workspace root".into());
    }
    let fuzz_workspace = root.join(WORKSPACE);
    let fuzz = workspace == fuzz_workspace;
    let mut by_id = BTreeMap::new();
    let mut from_registry = BTreeSet::new();
    let mut isolated = BTreeMap::new();
    let mut member = None;
    let mut gts = None;
    for package in slice(graph, "packages")? {
        let id = string(package, "id")?;
        let name = string(package, "name")?;
        if field(package, "source")?.as_str() == Some(REGISTRY) {
            from_registry.insert(id);
        }
        if by_id.insert(id, name).is_some() {
            return Err(format!("Cargo metadata repeats package ID {id}"));
        }
        if name == MEMBER {
            if !fuzz
                || member.replace(id).is_some()
                || !field(package, "source")?.is_null()
                || !slice(package, "publish")?.is_empty()
                || Path::new(string(package, "manifest_path")?) != workspace.join("Cargo.toml")
            {
                return Err(format!("unexpected GTS fuzz member {id}"));
            }
        } else if fuzz && name == GTS {
            if gts.replace(id).is_some()
                || !field(package, "source")?.is_null()
                || Path::new(string(package, "manifest_path")?)
                    != root.join("crates/gts/Cargo.toml")
            {
                return Err(format!("unexpected GTS path dependency {id}"));
            }
        } else if EDGES.iter().any(|edge| edge.package == name)
            && (!fuzz
                || isolated.insert(name, id).is_some()
                || string(package, "source")? != REGISTRY)
        {
            return Err(format!(
                "unexpected source, duplicate or graph for fuzz-only package {id}"
            ));
        }
    }

    if !fuzz {
        return Ok(());
    }
    let member = member.ok_or_else(|| "Cargo metadata has no GTS fuzz member".to_owned())?;
    let gts = gts.ok_or_else(|| "Cargo metadata has no GTS path dependency".to_owned())?;
    let resolve = field(graph, "resolve")?;
    let members = slice(graph, "workspace_members")?;
    if !matches!(members, [only] if only.as_str() == Some(member))
        || string(resolve, "root")? != member
        || isolated.len() != EDGES.len()
    {
        return Err("GTS fuzz metadata has the wrong workspace member or package set".into());
    }

    let mut found = BTreeSet::new();
    let mut children = BTreeMap::new();
    let mut nodes = BTreeSet::new();
    let mut found_gts = false;
    for node in slice(resolve, "nodes")? {
        let caller = string(node, "id")?;
        let parent = *by_id
            .get(caller)
            .ok_or_else(|| format!("Cargo metadata has unknown caller ID {caller}"))?;
        if !nodes.insert(caller) {
            return Err(format!("Cargo metadata repeats node ID {caller}"));
        }
        for dependency in slice(node, "deps")? {
            let destination = string(dependency, "pkg")?;
            let package = *by_id
                .get(destination)
                .ok_or_else(|| format!("Cargo metadata has unknown dependency ID {destination}"))?;
            children
                .entry(caller)
                .or_insert_with(Vec::new)
                .push(destination);
            if parent == "libfuzzer-sys" && !matches!(package, "cc" | "arbitrary") {
                return Err(format!("unapproved fuzz runtime dependency {package}"));
            }
            if destination == member {
                return Err(format!("unexpected incoming edge {parent} -> {MEMBER}"));
            }
            if caller == member && destination == gts {
                if found_gts || !matching_kind(dependency, None)? {
                    return Err(format!("unapproved dependency edge {MEMBER} -> {GTS}"));
                }
                found_gts = true;
            }
            if let Some(rule) = EDGES.iter().find(|edge| edge.package == package) {
                let expected_caller = if rule.parent == MEMBER {
                    Some(member)
                } else {
                    isolated.get(rule.parent).copied()
                };
                if expected_caller != Some(caller)
                    || !matching_kind(dependency, rule.kind)?
                    || !found.insert(package)
                {
                    return Err(format!("unapproved dependency edge {parent} -> {package}"));
                }
            }
        }
    }
    if !found_gts
        || !nodes.contains(gts)
        || found.len() != EDGES.len()
        || !nodes.contains(member)
        || isolated.values().any(|id| !nodes.contains(id))
    {
        return Err("GTS fuzz metadata is missing an approved dependency edge or node".into());
    }
    let cc = isolated
        .get("cc")
        .ok_or("GTS fuzz metadata has no cc build input")?;
    let mut seen = BTreeSet::new();
    let mut work = vec![*cc];
    while let Some(id) = work.pop() {
        if !seen.insert(id) {
            continue;
        }
        let name = by_id[id];
        if !from_registry.contains(id)
            || (!matches!(name, "cc" | "find-msvc-tools" | "jobserver" | "shlex")
                && !SHARED_HOST_INPUTS.contains(&name))
        {
            return Err(format!("unapproved cc host dependency {name}"));
        }
        if let Some(dependencies) = children.get(id) {
            work.extend(dependencies.iter().copied());
        }
    }
    Ok(())
}

/// Classify the exact fuzz graph provided by the locked dependency gate.
pub(crate) fn run(arguments: &[String]) -> Result<(), String> {
    let [_, root] = arguments else {
        return Err("expected --cargo-fuzz-edge-policy ROOT and Cargo metadata on stdin".into());
    };
    let root = Path::new(root)
        .canonicalize()
        .map_err(|cause| format!("cannot resolve policy root {root}: {cause}"))?;
    let text = std::io::read_to_string(std::io::stdin())
        .map_err(|cause| format!("cannot read Cargo metadata: {cause}"))?;
    let graph = purrdf_lex::json::read_slice(
        text.as_bytes(),
        purrdf_lex::json::Limits {
            unique_members: true,
            ..purrdf_lex::json::Limits::DEFAULT
        },
    )
    .map_err(|cause| format!("cannot parse Cargo metadata: {cause}"))?;
    validate(&graph, &root, &root.join(WORKSPACE))
}

#[cfg(test)]
mod tests {
    use super::{REGISTRY, WORKSPACE, validate};
    use std::path::Path;

    const EXACT: &str = r#"{
      "workspace_root":"/repo/crates/gts/fuzz",
      "workspace_members":["fuzz"],
      "packages":[
        {"id":"fuzz","name":"purrdf-gts-fuzz","source":null,"publish":[],"manifest_path":"/repo/crates/gts/fuzz/Cargo.toml"},
        {"id":"gts","name":"purrdf-gts","source":null,"manifest_path":"/repo/crates/gts/Cargo.toml"},
        {"id":"runtime","name":"libfuzzer-sys","source":"REGISTRY"},
        {"id":"arbitrary","name":"arbitrary","source":"REGISTRY"},
        {"id":"cc","name":"cc","source":"REGISTRY"},
        {"id":"msvc","name":"find-msvc-tools","source":"REGISTRY"},
        {"id":"jobs","name":"jobserver","source":"REGISTRY"},
        {"id":"shlex","name":"shlex","source":"REGISTRY"}
      ],
      "resolve":{"root":"fuzz","nodes":[
        {"id":"fuzz","deps":[
          {"pkg":"gts","dep_kinds":[{"kind":null,"target":null}]},
          {"pkg":"runtime","dep_kinds":[{"kind":null,"target":null}]}
        ]},
        {"id":"gts","deps":[]},
        {"id":"runtime","deps":[
          {"pkg":"arbitrary","dep_kinds":[{"kind":null,"target":null}]},
          {"pkg":"cc","dep_kinds":[{"kind":"build","target":null}]}
        ]},
        {"id":"arbitrary","deps":[]},
        {"id":"cc","deps":[
          {"pkg":"msvc","dep_kinds":[{"kind":null,"target":null}]},
          {"pkg":"jobs","dep_kinds":[{"kind":null,"target":null}]},
          {"pkg":"shlex","dep_kinds":[{"kind":null,"target":null}]}
        ]},
        {"id":"msvc","deps":[]},
        {"id":"jobs","deps":[]},
        {"id":"shlex","deps":[]}
      ]}
    }"#;

    fn case(contents: &str, workspace: &Path) -> Result<(), String> {
        let graph = purrdf_lex::json::read(&contents.replace("REGISTRY", REGISTRY))
            .map_err(|cause| cause.to_string())?;
        validate(&graph, Path::new("/repo"), workspace)
    }

    #[test]
    fn fuzz_policy_refuses_wrong_kinds_and_alternate_paths() {
        let workspace = Path::new("/repo").join(WORKSPACE);
        assert_eq!(case(EXACT, &workspace), Ok(()));
        for spelling in [r#""kind":null"#, r#""kind":"dev""#] {
            let changed = EXACT.replace(r#""kind":"build""#, spelling);
            assert_eq!(
                case(&changed, &workspace),
                Err("unapproved dependency edge libfuzzer-sys -> cc".into())
            );
        }
        for (id, name) in [("cc", "cc"), ("jobs", "jobserver")] {
            let changed = EXACT.replace(
                r#"{"id":"gts","deps":[]}"#,
                &format!(r#"{{"id":"gts","deps":[{{"pkg":"{id}","dep_kinds":[{{"kind":"build","target":null}}]}}]}}"#),
            );
            assert_eq!(
                case(&changed, &workspace),
                Err(format!("unapproved dependency edge purrdf-gts -> {name}"))
            );
        }
        assert!(
            case(
                &EXACT.replace("/repo/crates/gts/fuzz", "/repo"),
                Path::new("/repo")
            )
            .is_err()
        );
    }
}
