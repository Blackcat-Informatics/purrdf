// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Resolve ledger homes to workspace packages and render the consumer index.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use purrdf_lex::json::Value;

use crate::ledger::Job;

type Owners<'a> = BTreeMap<&'a str, BTreeSet<&'a str>>;

fn library_owners(metadata: &Value) -> Result<Owners<'_>, String> {
    let mut owners = Owners::new();
    for package in crate::policy::workspace_packages(metadata)? {
        let package = package?;
        let name = package["name"].as_str().ok_or("package has no name")?;
        let targets = package["targets"]
            .as_array()
            .ok_or_else(|| format!("package {name} has no targets array"))?;
        for target in targets {
            let kinds = target["kind"]
                .as_array()
                .ok_or_else(|| format!("package {name} has a target without a kind array"))?;
            let mut library = false;
            for kind in kinds {
                let kind = kind.as_str().ok_or("target kind is not a string")?;
                library |= matches!(
                    kind,
                    "lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro"
                );
            }
            if library {
                let target = target["name"]
                    .as_str()
                    .ok_or("library target has no name")?;
                owners.entry(target).or_default().insert(name);
            }
        }
    }
    Ok(owners)
}

fn owner<'a>(owners: &Owners<'a>, job: &Job) -> Result<&'a str, String> {
    let (target, _) = job
        .home
        .split_once("::")
        .filter(|(target, item)| !target.is_empty() && !item.is_empty())
        .ok_or_else(|| format!("job {} has an invalid home {}", job.id, job.home))?;
    let candidates = owners
        .get(target)
        .ok_or_else(|| format!("job {} has no workspace library target {target}", job.id))?;
    let mut names = candidates.iter().copied();
    match (names.next(), names.next()) {
        (Some(name), None) => Ok(name),
        _ => Err(format!(
            "job {} has ambiguous library target {target} in packages {}",
            job.id,
            candidates.iter().copied().collect::<Vec<_>>().join(", ")
        )),
    }
}

/// Render only after every ledger home resolves to one workspace package.
pub(crate) fn render(jobs: &[Job], metadata: &Value) -> Result<String, String> {
    let owners = library_owners(metadata)?;
    let mut by_package = BTreeMap::<&str, Vec<&Job>>::new();
    for job in jobs {
        by_package
            .entry(owner(&owners, job)?)
            .or_default()
            .push(job);
    }

    let mut page = String::from(concat!(
        "<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->\n",
        "<!-- SPDX-License-Identifier: CC-BY-4.0 -->\n",
        "<!-- Generated from helpers-ledger.toml. Do not edit. -->\n\n",
        "# Shared job homes\n\n",
        "Use a listed public entry point rather than implementing the same job. ",
        "An implementation home can be private, and not every owning package is ",
        "published. The [helper ledger](../helpers-ledger.toml) defines permitted ",
        "variants and the checks that reject local copies.\n\n",
        "Regenerate this reference with `make write-shared-job-homes`.\n",
    ));
    let format_error = |error| format!("write shared job homes: {error}");
    for (package, mut jobs) in by_package {
        jobs.sort_unstable_by_key(|job| &job.id);
        writeln!(page, "\n## `{package}`\n").map_err(format_error)?;
        for job in jobs {
            writeln!(page, "- `{}` — `{}`", job.id, job.home).map_err(format_error)?;
            if let Some((first, rest)) = job.entry_points.split_first() {
                write!(page, "  Additional entry points are `{first}`").map_err(format_error)?;
                for entry in rest {
                    write!(page, ", `{entry}`").map_err(format_error)?;
                }
                page.push_str(".\n");
            }
        }
    }
    Ok(page)
}

/// The destination for the generated reference.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Output {
    Stdout,
    Write(PathBuf),
    Check(PathBuf),
}

/// Generate, write or check the reference without traversing source targets.
pub(crate) fn run(root: &Path, jobs: &[Job], output: &Output) -> Result<(), String> {
    let metadata = crate::policy::workspace_metadata(root)?;
    let document = render(jobs, &metadata)?;
    let bytes = document.as_bytes();
    match output {
        Output::Stdout => std::io::stdout()
            .lock()
            .write_all(bytes)
            .map_err(|error| format!("write shared job homes: {error}")),
        Output::Write(path) => {
            std::fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
        }
        Output::Check(path) => {
            let artifact =
                std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
            if artifact != bytes {
                return Err(format!(
                    "{} is stale; run make write-shared-job-homes",
                    path.display()
                ));
            }
            Ok(())
        }
    }
}
