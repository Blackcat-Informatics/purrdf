// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admit repository Cargo config inputs and the exact locked graphs passed to Deny.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use purrdf_lex::json::{self, Value};
use purrdf_toml::{Kind, Node, Profile, Scalar, SourceDocument};

#[derive(Debug)]
pub(crate) enum Error {
    Usage,
    Read(PathBuf, io::Error),
    Profile(purrdf_toml::TomlError),
    Toml(PathBuf, purrdf_toml::TomlError),
    Json(PathBuf, json::Error),
    Invalid(PathBuf, &'static str),
    FuzzGraph(PathBuf, String),
    Outside {
        kind: &'static str,
        path: PathBuf,
        resolved: PathBuf,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(
                output,
                "expected --cargo-source-policy ROOT CONSUMER [CARGO_HOME ROOT_JSON CONSUMER_JSON]"
            ),
            Self::Read(path, cause) => write!(output, "{}: {cause}", path.display()),
            Self::Profile(cause) => write!(output, "Cargo TOML parser profile: {cause}"),
            Self::Toml(path, cause) => write!(output, "Cargo config {}: {cause}", path.display()),
            Self::Json(path, cause) => write!(output, "Cargo metadata {}: {cause}", path.display()),
            Self::Invalid(path, reason) => write!(output, "{}: {reason}", path.display()),
            Self::FuzzGraph(path, reason) => write!(output, "{}: {reason}", path.display()),
            Self::Outside {
                kind,
                path,
                resolved,
            } => write!(
                output,
                "{kind} is outside the policy root: {} -> {}",
                path.display(),
                resolved.display()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(_, cause) => Some(cause),
            Self::Profile(cause) | Self::Toml(_, cause) => Some(cause),
            Self::Json(_, cause) => Some(cause),
            _ => None,
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

pub(crate) fn run(arguments: &[String]) -> Result<()> {
    let [_, root, consumer, graphs @ ..] = arguments else {
        return Err(Error::Usage);
    };
    let root = canonical(Path::new(root))?;
    let relative = Path::new(consumer);
    if relative.as_os_str().is_empty()
        || !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(Error::Invalid(
            relative.to_owned(),
            "consumer must be a nonempty relative directory within the policy root",
        ));
    }
    let consumer = root.join(relative);
    match graphs {
        [] => check_configs(&root, &consumer),
        [cargo_home, root_graph, consumer_graph] => {
            let cargo_home = canonical(Path::new(cargo_home))?;
            check_graph(&root, &root, &cargo_home, Path::new(root_graph))?;
            check_graph(&root, &consumer, &cargo_home, Path::new(consumer_graph))
        }
        _ => Err(Error::Usage),
    }
}

fn canonical(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .map_err(|cause| Error::Read(path.to_owned(), cause))
}

fn contain(root: &Path, path: &Path, resolved: PathBuf, kind: &'static str) -> Result<PathBuf> {
    if resolved.starts_with(root) {
        Ok(resolved)
    } else {
        Err(Error::Outside {
            kind,
            path: path.to_owned(),
            resolved,
        })
    }
}

fn check_configs(root: &Path, consumer: &Path) -> Result<()> {
    let profile = Profile::standard().map_err(Error::Profile)?;
    for directory in consumer
        .ancestors()
        .take_while(|path| path.starts_with(root))
    {
        for name in ["config", "config.toml"] {
            let path = directory.join(".cargo").join(name);
            if path
                .try_exists()
                .map_err(|cause| Error::Read(path.clone(), cause))?
            {
                check_config_tree(root, path, &profile)?;
            }
        }
    }
    Ok(())
}

enum Visit {
    Read { path: PathBuf, optional: bool },
    Leave(PathBuf),
}

fn check_config_tree(root: &Path, path: PathBuf, profile: &Profile) -> Result<()> {
    let mut active = BTreeSet::new();
    let mut pending = vec![Visit::Read {
        path,
        optional: false,
    }];
    while let Some(visit) = pending.pop() {
        let (path, optional) = match visit {
            Visit::Read { path, optional } => (path, optional),
            Visit::Leave(path) => {
                active.remove(&path);
                continue;
            }
        };
        let Some(resolved) = config_path(root, &path, optional)? else {
            continue;
        };
        if !active.insert(resolved.clone()) {
            return Err(Error::Invalid(path, "Cargo config include cycle"));
        }
        pending.push(Visit::Leave(resolved.clone()));
        read_includes(&path, &resolved, profile, &mut pending)?;
    }
    Ok(())
}

fn config_path(root: &Path, path: &Path, optional: bool) -> Result<Option<PathBuf>> {
    const KIND: &str = "Cargo config include";
    match path.canonicalize() {
        Ok(resolved) => contain(root, path, resolved, KIND).map(Some),
        Err(cause) if optional && cause.kind() == io::ErrorKind::NotFound => {
            // An absent optional file is permitted only below an admitted
            // existing ancestor. Never mistake a dangling link for absence.
            for ancestor in path.ancestors() {
                match fs::symlink_metadata(ancestor) {
                    Ok(_) => {
                        contain(root, path, canonical(ancestor)?, KIND)?;
                        return Ok(None);
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(Error::Read(ancestor.to_owned(), error)),
                }
            }
            Err(Error::Read(path.to_owned(), cause))
        }
        Err(cause) => Err(Error::Read(path.to_owned(), cause)),
    }
}

fn read_includes(
    path: &Path,
    resolved: &Path,
    profile: &Profile,
    pending: &mut Vec<Visit>,
) -> Result<()> {
    if !resolved.is_file() {
        return Err(Error::Invalid(
            path.to_owned(),
            "Cargo config include must be a regular file",
        ));
    }
    let bytes = fs::read(resolved).map_err(|cause| Error::Read(path.to_owned(), cause))?;
    let document = purrdf_toml::analyze(
        SourceDocument {
            id: "urn:example:cargo-config",
            bytes: &bytes,
        },
        profile,
    )
    .map_err(|cause| Error::Toml(path.to_owned(), cause))?;
    let nodes = document.nodes();
    let Some(include) = child(nodes, &nodes[0], "include") else {
        return Ok(());
    };
    let invalid = || Error::Invalid(path.to_owned(), "invalid Cargo config include");
    if !matches!(include.kind, Kind::Array | Kind::ArrayOfTables) {
        return Err(invalid());
    }
    let parent = path.parent().ok_or_else(invalid)?;
    for &index in include.children.iter().rev() {
        let entry = &nodes[index];
        let (name, optional) = match &entry.kind {
            Kind::Scalar(Scalar::String(name)) => (name, false),
            Kind::Table(_) => {
                let Some(Kind::Scalar(Scalar::String(name))) =
                    child(nodes, entry, "path").map(|node| &node.kind)
                else {
                    return Err(invalid());
                };
                let optional = match child(nodes, entry, "optional").map(|node| &node.kind) {
                    None => false,
                    Some(Kind::Scalar(Scalar::Boolean(value))) => *value,
                    _ => return Err(invalid()),
                };
                (name, optional)
            }
            _ => return Err(invalid()),
        };
        pending.push(Visit::Read {
            path: parent.join(name),
            optional,
        });
    }
    Ok(())
}

fn child<'a>(nodes: &'a [Node], parent: &Node, key: &str) -> Option<&'a Node> {
    parent
        .children
        .iter()
        .map(|&index| &nodes[index])
        .find(|node| node.key.as_ref().is_some_and(|name| name.decoded == key))
}

fn check_graph(root: &Path, expected: &Path, cargo_home: &Path, metadata: &Path) -> Result<()> {
    let bytes = fs::read(metadata).map_err(|cause| Error::Read(metadata.to_owned(), cause))?;
    let graph = json::read_slice(
        &bytes,
        json::Limits {
            unique_members: true,
            ..json::Limits::DEFAULT
        },
    )
    .map_err(|cause| Error::Json(metadata.to_owned(), cause))?;
    let invalid = || {
        Error::Invalid(
            metadata.to_owned(),
            "invalid or incomplete Cargo metadata graph",
        )
    };
    let workspace = graph
        .get("workspace_root")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    if graph_path(root, workspace, "Cargo metadata workspace root", true)? != expected
        || graph.get("resolve").and_then(Value::as_object).is_none()
    {
        return Err(Error::Invalid(
            metadata.to_owned(),
            "Cargo metadata has an unexpected workspace or incomplete graph",
        ));
    }
    let packages = graph
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    crate::cargo_fuzz_policy::validate(&graph, root, expected)
        .map_err(|reason| Error::FuzzGraph(metadata.to_owned(), reason))?;
    let registry = cargo_home.join("registry/src");
    let git = cargo_home.join("git/checkouts");
    for package in packages {
        let source = package.get("source").ok_or_else(invalid)?;
        let (package_root, manifest_kind, dependency_kind) = if source.is_null() {
            (
                root,
                "Cargo metadata local package manifest",
                "Cargo metadata local dependency path",
            )
        } else {
            match source.as_str().ok_or_else(invalid)? {
                source if source.starts_with("registry+") || source.starts_with("sparse+") => (
                    registry.as_path(),
                    "Cargo metadata registry package manifest",
                    "Cargo metadata registry dependency path",
                ),
                source if source.starts_with("git+") => (
                    git.as_path(),
                    "Cargo metadata Git package manifest",
                    "Cargo metadata Git dependency path",
                ),
                _ => {
                    return Err(Error::Invalid(
                        metadata.to_owned(),
                        "unexpected Cargo metadata package source",
                    ));
                }
            }
        };
        let manifest = package
            .get("manifest_path")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        graph_path(package_root, manifest, manifest_kind, false)?;
        let dependencies = package
            .get("dependencies")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?;
        for dependency in dependencies {
            if let Some(value) = dependency.get("path").filter(|value| !value.is_null()) {
                let path = value.as_str().ok_or_else(invalid)?;
                graph_path(package_root, path, dependency_kind, true)?;
            }
        }
    }
    Ok(())
}

fn graph_path(root: &Path, path: &str, kind: &'static str, directory: bool) -> Result<PathBuf> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err(Error::Invalid(
            path.to_owned(),
            "Cargo metadata path must be absolute",
        ));
    }
    let resolved = contain(root, path, canonical(path)?, kind)?;
    if if directory {
        resolved.is_dir()
    } else {
        resolved.is_file()
    } {
        Ok(resolved)
    } else {
        Err(Error::Invalid(
            path.to_owned(),
            "Cargo metadata path has the wrong file type",
        ))
    }
}
