// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Keep the GTS fuzz runtime and its native build inputs out of shipped graphs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::{fmt, io};

use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_lex::json::{self, Value};

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

#[derive(Debug)]
pub(crate) enum Error {
    Usage,
    ResolveRoot {
        root: PathBuf,
        cause: io::Error,
    },
    ReadMetadata(io::Error),
    ParseMetadata(json::Error),
    MissingField {
        field: &'static str,
    },
    InvalidFieldType {
        field: &'static str,
        expected: FieldType,
    },
    UnexpectedWorkspace {
        actual: PathBuf,
        expected: PathBuf,
    },
    DuplicatePackageId {
        id: Box<str>,
    },
    UnexpectedMember {
        id: Box<str>,
    },
    UnexpectedGtsPath {
        id: Box<str>,
    },
    UnexpectedFuzzPackage {
        id: Box<str>,
        package: Box<str>,
    },
    MissingMember,
    MissingGts,
    WrongWorkspaceMembers {
        expected_id: Box<str>,
        actual_count: usize,
        actual_id: Option<Box<str>>,
    },
    WrongResolveRoot {
        expected_id: Box<str>,
        actual_id: Box<str>,
    },
    MissingIsolatedPackage {
        package: &'static str,
    },
    UnknownCallerId {
        id: Box<str>,
    },
    DuplicateNodeId {
        id: Box<str>,
    },
    UnknownDependencyId {
        id: Box<str>,
    },
    RejectedEdge {
        kind: EdgeFailure,
        parent: Box<str>,
        package: Box<str>,
        caller_id: Box<str>,
        dependency_id: Box<str>,
    },
    MissingApprovedEdge {
        parent: &'static str,
        package: &'static str,
    },
    MissingApprovedNode {
        id: Box<str>,
    },
    MissingCc,
    UnapprovedHostDependency {
        id: Box<str>,
        package: Box<str>,
    },
}

#[derive(Debug)]
pub(crate) enum FieldType {
    String,
    Array,
}

impl FieldType {
    fn code(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Array => "array",
        }
    }
}

#[derive(Debug)]
pub(crate) enum EdgeFailure {
    RuntimeDependency,
    IncomingMember,
    Unapproved,
}

impl EdgeFailure {
    fn code(&self) -> &'static str {
        match self {
            Self::RuntimeDependency => "runtime-dependency",
            Self::IncomingMember => "incoming-member",
            Self::Unapproved => "unapproved",
        }
    }
}

impl Error {
    fn edge(
        kind: EdgeFailure,
        parent: &str,
        package: &str,
        caller: &str,
        dependency: &str,
    ) -> Self {
        Self::RejectedEdge {
            kind,
            parent: parent.into(),
            package: package.into(),
            caller_id: caller.into(),
            dependency_id: dependency.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => output
                .write_str("expected --cargo-fuzz-edge-policy ROOT and Cargo metadata on stdin"),
            Self::ResolveRoot { root, cause } => write!(
                output,
                "cannot resolve policy root {}: {cause}",
                root.display()
            ),
            Self::ReadMetadata(cause) => write!(output, "cannot read Cargo metadata: {cause}"),
            Self::ParseMetadata(cause) => write!(output, "cannot parse Cargo metadata: {cause}"),
            Self::MissingField { field } => write!(output, "Cargo metadata is missing {field}"),
            Self::InvalidFieldType { field, expected } => {
                let description = match expected {
                    FieldType::String => "a string",
                    FieldType::Array => "an array",
                };
                write!(output, "Cargo metadata {field} must be {description}")
            }
            Self::UnexpectedWorkspace { .. } => {
                output.write_str("Cargo metadata has an unexpected workspace root")
            }
            Self::DuplicatePackageId { id } => {
                write!(output, "Cargo metadata repeats package ID {id}")
            }
            Self::UnexpectedMember { id } => write!(output, "unexpected GTS fuzz member {id}"),
            Self::UnexpectedGtsPath { id } => write!(output, "unexpected GTS path dependency {id}"),
            Self::UnexpectedFuzzPackage { id, .. } => write!(
                output,
                "unexpected source, duplicate or graph for fuzz-only package {id}"
            ),
            Self::MissingMember => output.write_str("Cargo metadata has no GTS fuzz member"),
            Self::MissingGts => output.write_str("Cargo metadata has no GTS path dependency"),
            Self::WrongWorkspaceMembers { .. }
            | Self::WrongResolveRoot { .. }
            | Self::MissingIsolatedPackage { .. } => {
                output.write_str("GTS fuzz metadata has the wrong workspace member or package set")
            }
            Self::UnknownCallerId { id } => {
                write!(output, "Cargo metadata has unknown caller ID {id}")
            }
            Self::DuplicateNodeId { id } => write!(output, "Cargo metadata repeats node ID {id}"),
            Self::UnknownDependencyId { id } => {
                write!(output, "Cargo metadata has unknown dependency ID {id}")
            }
            Self::RejectedEdge {
                kind,
                parent,
                package,
                ..
            } => match kind {
                EdgeFailure::RuntimeDependency => {
                    write!(output, "unapproved fuzz runtime dependency {package}")
                }
                EdgeFailure::IncomingMember => {
                    write!(output, "unexpected incoming edge {parent} -> {package}")
                }
                EdgeFailure::Unapproved => {
                    write!(output, "unapproved dependency edge {parent} -> {package}")
                }
            },
            Self::MissingApprovedEdge { .. } | Self::MissingApprovedNode { .. } => {
                output.write_str("GTS fuzz metadata is missing an approved dependency edge or node")
            }
            Self::MissingCc => output.write_str("GTS fuzz metadata has no cc build input"),
            Self::UnapprovedHostDependency { package, .. } => {
                write!(output, "unapproved cc host dependency {package}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ResolveRoot { cause, .. } | Self::ReadMetadata(cause) => Some(cause),
            Self::ParseMetadata(cause) => Some(cause),
            _ => None,
        }
    }
}

impl StructuredError for Error {
    purrdf_hash::error_visit! {
        Usage => "cargo-fuzz-usage" {},
        ResolveRoot { root, cause } => "cargo-fuzz-resolve-root" {
            "root" => Bytes(root.as_os_str().as_encoded_bytes()),
            "cause" => Cause(cause),
        },
        ReadMetadata(cause) => "cargo-fuzz-read-metadata" { "cause" => Cause(cause) },
        ParseMetadata(cause) => "cargo-fuzz-parse-metadata" { "cause" => Cause(cause) },
        MissingField { field } => "cargo-fuzz-missing-field" { "field" => Code(field) },
        InvalidFieldType { field, expected } => "cargo-fuzz-invalid-field-type" {
            "field" => Code(field),
            "expected" => Code(expected.code()),
        },
        UnexpectedWorkspace { actual, expected } => "cargo-fuzz-unexpected-workspace" {
            "actual" => Bytes(actual.as_os_str().as_encoded_bytes()),
            "expected" => Bytes(expected.as_os_str().as_encoded_bytes()),
        },
        DuplicatePackageId { id } => "cargo-fuzz-duplicate-package-id" { "id" => Text(id) },
        UnexpectedMember { id } => "cargo-fuzz-unexpected-member" { "id" => Text(id) },
        UnexpectedGtsPath { id } => "cargo-fuzz-unexpected-gts-path" { "id" => Text(id) },
        UnexpectedFuzzPackage { id, package } => "cargo-fuzz-unexpected-package" {
            "id" => Text(id),
            "package" => Text(package),
        },
        MissingMember => "cargo-fuzz-missing-member" {},
        MissingGts => "cargo-fuzz-missing-gts" {},
        WrongWorkspaceMembers { expected_id, actual_count, actual_id } => "cargo-fuzz-wrong-workspace-members" {
            "expected_id" => Text(expected_id),
            "actual_count" => Unsigned(*actual_count as u128),
            "actual_id" => Value(actual_id.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
        },
        WrongResolveRoot { expected_id, actual_id } => "cargo-fuzz-wrong-resolve-root" {
            "expected_id" => Text(expected_id),
            "actual_id" => Text(actual_id),
        },
        MissingIsolatedPackage { package } => "cargo-fuzz-missing-isolated-package" {
            "package" => Text(package),
        },
        UnknownCallerId { id } => "cargo-fuzz-unknown-caller-id" { "id" => Text(id) },
        DuplicateNodeId { id } => "cargo-fuzz-duplicate-node-id" { "id" => Text(id) },
        UnknownDependencyId { id } => "cargo-fuzz-unknown-dependency-id" { "id" => Text(id) },
        RejectedEdge { kind, parent, package, caller_id, dependency_id } => "cargo-fuzz-rejected-edge" {
            "kind" => Code(kind.code()),
            "parent" => Text(parent),
            "package" => Text(package),
            "caller_id" => Text(caller_id),
            "dependency_id" => Text(dependency_id),
        },
        MissingApprovedEdge { parent, package } => "cargo-fuzz-missing-approved-edge" {
            "parent" => Text(parent),
            "package" => Text(package),
        },
        MissingApprovedNode { id } => "cargo-fuzz-missing-approved-node" { "id" => Text(id) },
        MissingCc => "cargo-fuzz-missing-cc" {},
        UnapprovedHostDependency { id, package } => "cargo-fuzz-unapproved-host-dependency" {
            "id" => Text(id),
            "package" => Text(package),
        },
    }
}

fn field<'a>(value: &'a Value, name: &'static str) -> Result<&'a Value, Error> {
    value.get(name).ok_or(Error::MissingField { field: name })
}

fn string<'a>(value: &'a Value, name: &'static str) -> Result<&'a str, Error> {
    field(value, name)?.as_str().ok_or(Error::InvalidFieldType {
        field: name,
        expected: FieldType::String,
    })
}

fn slice<'a>(value: &'a Value, name: &'static str) -> Result<&'a [Value], Error> {
    field(value, name)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or(Error::InvalidFieldType {
            field: name,
            expected: FieldType::Array,
        })
}

fn matching_kind(dep: &Value, expected: Option<&str>) -> Result<bool, Error> {
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
pub(crate) fn validate(graph: &Value, root: &Path, workspace: &Path) -> Result<(), Error> {
    let actual = Path::new(string(graph, "workspace_root")?);
    if actual != workspace {
        return Err(Error::UnexpectedWorkspace {
            actual: actual.into(),
            expected: workspace.into(),
        });
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
            return Err(Error::DuplicatePackageId { id: id.into() });
        }
        if name == MEMBER {
            if !fuzz
                || member.replace(id).is_some()
                || !field(package, "source")?.is_null()
                || !slice(package, "publish")?.is_empty()
                || Path::new(string(package, "manifest_path")?) != workspace.join("Cargo.toml")
            {
                return Err(Error::UnexpectedMember { id: id.into() });
            }
        } else if fuzz && name == GTS {
            if gts.replace(id).is_some()
                || !field(package, "source")?.is_null()
                || Path::new(string(package, "manifest_path")?)
                    != root.join("crates/gts/Cargo.toml")
            {
                return Err(Error::UnexpectedGtsPath { id: id.into() });
            }
        } else if EDGES.iter().any(|edge| edge.package == name)
            && (!fuzz
                || isolated.insert(name, id).is_some()
                || string(package, "source")? != REGISTRY)
        {
            return Err(Error::UnexpectedFuzzPackage {
                id: id.into(),
                package: name.into(),
            });
        }
    }

    if !fuzz {
        return Ok(());
    }
    let member = member.ok_or(Error::MissingMember)?;
    let gts = gts.ok_or(Error::MissingGts)?;
    let resolve = field(graph, "resolve")?;
    let members = slice(graph, "workspace_members")?;
    if !matches!(members, [only] if only.as_str() == Some(member)) {
        return Err(Error::WrongWorkspaceMembers {
            expected_id: member.into(),
            actual_count: members.len(),
            actual_id: match members {
                [only] => only.as_str().map(Into::into),
                _ => None,
            },
        });
    }
    let actual_root = string(resolve, "root")?;
    if actual_root != member {
        return Err(Error::WrongResolveRoot {
            expected_id: member.into(),
            actual_id: actual_root.into(),
        });
    }
    if let Some(rule) = EDGES
        .iter()
        .find(|rule| !isolated.contains_key(rule.package))
    {
        return Err(Error::MissingIsolatedPackage {
            package: rule.package,
        });
    }

    let mut found = BTreeSet::new();
    let mut children = BTreeMap::new();
    let mut nodes = BTreeSet::new();
    let mut found_gts = false;
    for node in slice(resolve, "nodes")? {
        let caller = string(node, "id")?;
        let parent = *by_id
            .get(caller)
            .ok_or_else(|| Error::UnknownCallerId { id: caller.into() })?;
        if !nodes.insert(caller) {
            return Err(Error::DuplicateNodeId { id: caller.into() });
        }
        for dependency in slice(node, "deps")? {
            let destination = string(dependency, "pkg")?;
            let package = *by_id
                .get(destination)
                .ok_or_else(|| Error::UnknownDependencyId {
                    id: destination.into(),
                })?;
            children
                .entry(caller)
                .or_insert_with(Vec::new)
                .push(destination);
            if parent == "libfuzzer-sys" && !matches!(package, "cc" | "arbitrary") {
                return Err(Error::edge(
                    EdgeFailure::RuntimeDependency,
                    parent,
                    package,
                    caller,
                    destination,
                ));
            }
            if destination == member {
                return Err(Error::edge(
                    EdgeFailure::IncomingMember,
                    parent,
                    MEMBER,
                    caller,
                    destination,
                ));
            }
            if caller == member && destination == gts {
                if found_gts || !matching_kind(dependency, None)? {
                    return Err(Error::edge(
                        EdgeFailure::Unapproved,
                        MEMBER,
                        GTS,
                        caller,
                        destination,
                    ));
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
                    return Err(Error::edge(
                        EdgeFailure::Unapproved,
                        parent,
                        package,
                        caller,
                        destination,
                    ));
                }
            }
        }
    }
    if !found_gts {
        return Err(Error::MissingApprovedEdge {
            parent: MEMBER,
            package: GTS,
        });
    }
    if !nodes.contains(gts) {
        return Err(Error::MissingApprovedNode { id: gts.into() });
    }
    if let Some(rule) = EDGES.iter().find(|rule| !found.contains(rule.package)) {
        return Err(Error::MissingApprovedEdge {
            parent: rule.parent,
            package: rule.package,
        });
    }
    if !nodes.contains(member) {
        return Err(Error::MissingApprovedNode { id: member.into() });
    }
    if let Some(&id) = isolated.values().find(|&&id| !nodes.contains(id)) {
        return Err(Error::MissingApprovedNode { id: id.into() });
    }
    let cc = isolated.get("cc").ok_or(Error::MissingCc)?;
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
            return Err(Error::UnapprovedHostDependency {
                id: id.into(),
                package: name.into(),
            });
        }
        if let Some(dependencies) = children.get(id) {
            work.extend(dependencies.iter().copied());
        }
    }
    Ok(())
}

/// Classify the exact fuzz graph provided by the locked dependency gate.
pub(crate) fn run(arguments: &[String]) -> Result<(), Error> {
    let [_, root] = arguments else {
        return Err(Error::Usage);
    };
    let root = Path::new(root)
        .canonicalize()
        .map_err(|cause| Error::ResolveRoot {
            root: PathBuf::from(root),
            cause,
        })?;
    let text = io::read_to_string(io::stdin()).map_err(Error::ReadMetadata)?;
    let graph = json::read_slice(
        text.as_bytes(),
        json::Limits {
            unique_members: true,
            ..json::Limits::DEFAULT
        },
    )
    .map_err(Error::ParseMetadata)?;
    validate(&graph, &root, &root.join(WORKSPACE))
}

#[cfg(test)]
mod tests {
    use super::{Error, REGISTRY, WORKSPACE, validate};
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

    fn case(contents: &str, workspace: &Path) -> Result<(), Error> {
        let graph = purrdf_lex::json::read(&contents.replace("REGISTRY", REGISTRY))
            .map_err(Error::ParseMetadata)?;
        validate(&graph, Path::new("/repo"), workspace)
    }

    #[test]
    fn fuzz_policy_refuses_wrong_kinds_and_alternate_paths() {
        let workspace = Path::new("/repo").join(WORKSPACE);
        case(EXACT, &workspace).expect("approved graph");
        let missing = EXACT.replace(r#"{"id":"jobs","deps":[]},"#, "");
        let error = case(&missing, &workspace).expect_err("missing approved node");
        assert_eq!(
            error.to_string(),
            "GTS fuzz metadata is missing an approved dependency edge or node"
        );
        let captured = purrdf_testkit::error_capture::Capture::read(&error);
        assert_eq!(captured.code, "cargo-fuzz-missing-approved-node");
        assert!(matches!(
            captured.field("id"),
            purrdf_hash::error::ErrorValue::Text("jobs")
        ));
        for spelling in [r#""kind":null"#, r#""kind":"dev""#] {
            let changed = EXACT.replace(r#""kind":"build""#, spelling);
            let error = case(&changed, &workspace).expect_err("unapproved cc kind");
            assert_eq!(
                error.to_string(),
                "unapproved dependency edge libfuzzer-sys -> cc"
            );
        }
        for (id, name) in [("cc", "cc"), ("jobs", "jobserver")] {
            let changed = EXACT.replace(
                r#"{"id":"gts","deps":[]}"#,
                &format!(r#"{{"id":"gts","deps":[{{"pkg":"{id}","dep_kinds":[{{"kind":"build","target":null}}]}}]}}"#),
            );
            let error = case(&changed, &workspace).expect_err("unapproved incoming edge");
            assert_eq!(
                error.to_string(),
                format!("unapproved dependency edge purrdf-gts -> {name}")
            );
            let captured = purrdf_testkit::error_capture::Capture::read(&error);
            assert_eq!(captured.code, "cargo-fuzz-rejected-edge");
            assert!(
                matches!(captured.field("dependency_id"), purrdf_hash::error::ErrorValue::Text(actual) if actual == id)
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
