// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Grouping, ledger matching and the reports built from them.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value as Json, json};

use crate::ledger::{Job, Ledger};
use crate::normalize::MIN_TOKENS;
use crate::source::{Unit, UnitKind, Workspace};

/// A set of units with one fingerprint.
#[derive(Debug)]
pub(crate) struct Group {
    /// The shared fingerprint id.
    pub(crate) fingerprint: String,
    /// Tokens in the shared structural form (0 for a shim group).
    pub(crate) tokens: usize,
    /// Indexes into [`Workspace::units`].
    pub(crate) members: Vec<usize>,
}

/// Groups of two or more units sharing a structural fingerprint, largest first.
pub(crate) fn isomorphic_groups(workspace: &Workspace) -> Vec<Group> {
    let mut by_print: BTreeMap<&str, (usize, Vec<usize>)> = BTreeMap::new();
    for (index, unit) in workspace.units.iter().enumerate() {
        if let Some(print) = &unit.print
            && print.tokens >= MIN_TOKENS
        {
            by_print
                .entry(print.structural.as_str())
                .or_insert_with(|| (print.tokens, Vec::new()))
                .1
                .push(index);
        }
    }
    collect_groups(by_print)
}

/// Groups of two or more thin forwarders sharing a shim fingerprint.
pub(crate) fn shim_groups(workspace: &Workspace) -> Vec<Group> {
    let mut by_print: BTreeMap<&str, (usize, Vec<usize>)> = BTreeMap::new();
    for (index, unit) in workspace.units.iter().enumerate() {
        if let Some(shim) = unit.print.as_ref().and_then(|print| print.shim.as_deref()) {
            by_print
                .entry(shim)
                .or_insert_with(|| (0, Vec::new()))
                .1
                .push(index);
        }
    }
    collect_groups(by_print)
}

fn collect_groups(by_print: BTreeMap<&str, (usize, Vec<usize>)>) -> Vec<Group> {
    let mut groups: Vec<Group> = by_print
        .into_iter()
        .filter(|(_, (_, members))| members.len() >= 2)
        .map(|(fingerprint, (tokens, members))| Group {
            fingerprint: fingerprint.to_owned(),
            tokens,
            members,
        })
        .collect();
    groups.sort_by(|left, right| {
        right
            .members
            .len()
            .cmp(&left.members.len())
            .then(right.tokens.cmp(&left.tokens))
            .then(left.fingerprint.cmp(&right.fingerprint))
    });
    groups
}

/// Every variant symbol exempt from `detector`, across the ledger.
pub(crate) fn variant_symbols(
    ledger: &Ledger,
    workspace: &Workspace,
    detector: &str,
) -> BTreeSet<String> {
    ledger
        .jobs
        .iter()
        .flat_map(|job| job.variants.iter())
        .filter(|variant| variant.detector == detector)
        .map(|variant| {
            workspace
                .resolve(&variant.symbol)
                .unwrap_or_else(|| variant.symbol.clone())
        })
        .collect()
}

/// Whether a group is sanctioned: every member but one is an `isomorphic` variant.
pub(crate) fn group_is_sanctioned(
    group: &Group,
    workspace: &Workspace,
    variants: &BTreeSet<String>,
) -> bool {
    let unsanctioned = group
        .members
        .iter()
        .filter(|&&index| !variants.contains(&workspace.units[index].symbol))
        .count();
    unsanctioned <= 1
}

/// One unit or rule hit that matches a job's forbidden set.
#[derive(Debug)]
pub(crate) struct Match {
    /// The matching symbol (a rule hit's is its file and enclosing items).
    pub(crate) symbol: String,
    /// The package that holds it.
    pub(crate) package: String,
    /// The repo-relative file.
    pub(crate) file: String,
    /// 1-based line.
    pub(crate) line: usize,
    /// What matched.
    pub(crate) reasons: Vec<String>,
    /// Whether the unit is in the job's home package. A rule hit is only when
    /// its rule exempts the home (a hex-digit table); every other rule forbids
    /// its pattern everywhere, the home included.
    pub(crate) in_home: bool,
    /// Whether the unit is a `forbidden` variant of this job.
    pub(crate) variant: bool,
}

/// A job's home, resolved.
#[derive(Debug)]
pub(crate) struct Home {
    /// The defining symbol.
    pub(crate) symbol: String,
    /// The package it lives in.
    pub(crate) package: String,
}

/// Resolve a job's home, or say why it does not resolve.
pub(crate) fn home(job: &Job, workspace: &Workspace) -> Result<Home, String> {
    let symbol = workspace.resolve(&job.home).ok_or_else(|| {
        format!(
            "job `{}`: home `{}` does not resolve to a defined item",
            job.id, job.home
        )
    })?;
    let package = workspace.definitions[&symbol].package.clone();
    Ok(Home { symbol, package })
}

/// Every unit matching `job`'s forbidden set.
pub(crate) fn matches(job: &Job, workspace: &Workspace, home_package: &str) -> Vec<Match> {
    let variants: BTreeSet<String> = job
        .variants
        .iter()
        .filter(|variant| variant.detector == "forbidden")
        .map(|variant| {
            workspace
                .resolve(&variant.symbol)
                .unwrap_or_else(|| variant.symbol.clone())
        })
        .collect();
    // A rule hit is named by its file and enclosing items, not by a resolved
    // path, so a `forbidden` variant sanctions it by its file and item name.
    let rule_variants: BTreeSet<(&str, &str)> = job
        .variants
        .iter()
        .filter(|variant| variant.detector == "forbidden")
        .map(|variant| {
            let item = variant
                .symbol
                .rsplit_once("::")
                .map_or(variant.symbol.as_str(), |(_, item)| item);
            (variant.file.as_str(), item)
        })
        .collect();
    let mut found = Vec::new();
    for unit in &workspace.units {
        let reasons = unit_reasons(job, unit);
        if !reasons.is_empty() {
            found.push(Match {
                symbol: unit.symbol.clone(),
                package: unit.package.clone(),
                file: unit.file.clone(),
                line: unit.line,
                reasons,
                in_home: unit.package == home_package,
                variant: variants.contains(&unit.symbol),
            });
        }
    }
    for hit in &workspace.rule_hits {
        if job.forbidden.fingerprints.iter().any(|id| id == hit.rule) {
            found.push(Match {
                symbol: hit.symbol.clone(),
                package: hit.package.clone(),
                file: hit.file.clone(),
                line: hit.line,
                reasons: vec![format!("{}: {}", hit.rule, hit.detail)],
                in_home: hit.home_exempt && hit.package == home_package,
                variant: variants.contains(&hit.symbol)
                    || hit.symbol.rsplit_once("::").is_some_and(|(_, item)| {
                        rule_variants.contains(&(hit.file.as_str(), item))
                    }),
            });
        }
    }
    found
}

fn unit_reasons(job: &Job, unit: &Unit) -> Vec<String> {
    let mut reasons = Vec::new();
    if unit.kind == UnitKind::Function {
        for pattern in &job.forbidden.names {
            if pattern.is_match(&unit.name) {
                reasons.push(format!("name /{}/", pattern.as_str()));
            }
        }
    }
    for constant in &job.forbidden.constants {
        if unit.constants.contains(constant) {
            reasons.push(format!("constant {constant:#x}"));
        }
    }
    for id in &job.forbidden.fingerprints {
        let body = unit
            .print
            .as_ref()
            .is_some_and(|print| &print.structural == id || print.shim.as_ref() == Some(id));
        if body || unit.tables.contains(id.as_str()) {
            reasons.push(format!("fingerprint {id}"));
        }
    }
    reasons
}

/// The copies `--count` reports: matches outside the home that are not variants.
pub(crate) fn copies(found: &[Match]) -> usize {
    found
        .iter()
        .filter(|found| !found.in_home && !found.variant)
        .count()
}

fn unit_json(unit: &Unit) -> Json {
    json!({
        "symbol": unit.symbol,
        "package": unit.package,
        "file": unit.file,
        "line": unit.line,
    })
}

fn group_json(group: &Group, workspace: &Workspace, variants: &BTreeSet<String>) -> Json {
    json!({
        "fingerprint": group.fingerprint,
        "tokens": group.tokens,
        "sanctioned": group_is_sanctioned(group, workspace, variants),
        "members": group
            .members
            .iter()
            .map(|&index| unit_json(&workspace.units[index]))
            .collect::<Vec<_>>(),
    })
}

/// The baseline document: every group, every hex table, and every large integer
/// constant that appears in more than one package.
pub(crate) fn baseline(workspace: &Workspace, ledger: &Ledger) -> Json {
    let variants = variant_symbols(ledger, workspace, "isomorphic");
    let groups = isomorphic_groups(workspace);
    let shims = shim_groups(workspace);
    let tables: Vec<Json> = workspace
        .units
        .iter()
        .filter(|unit| !unit.tables.is_empty())
        .map(|unit| {
            let mut entry = unit_json(unit);
            entry["tables"] = json!(unit.tables.iter().collect::<Vec<_>>());
            entry
        })
        .collect();
    let mut constants: BTreeMap<u128, BTreeMap<&str, Vec<&Unit>>> = BTreeMap::new();
    for unit in &workspace.units {
        for &value in &unit.constants {
            if value > 0xffff {
                constants
                    .entry(value)
                    .or_default()
                    .entry(unit.package.as_str())
                    .or_default()
                    .push(unit);
            }
        }
    }
    let shared: Vec<Json> = constants
        .into_iter()
        .filter(|(_, packages)| packages.len() >= 2)
        .map(|(value, packages)| {
            json!({
                "value": format!("{value:#x}"),
                "packages": packages.len(),
                "units": packages
                    .values()
                    .flatten()
                    .map(|unit| unit_json(unit))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "min_tokens": MIN_TOKENS,
        "units": workspace.units.len(),
        "isomorphic_groups": groups.iter().map(|group| group_json(group, workspace, &variants)).collect::<Vec<_>>(),
        "shim_groups": shims.iter().map(|group| group_json(group, workspace, &variants)).collect::<Vec<_>>(),
        "hex_tables": tables,
        "shared_constants": shared,
    })
}

/// The document `scripts/check-shared-helpers.py` reads.
pub(crate) fn index(workspace: &Workspace, ledger: &Ledger) -> Json {
    let mut symbols = serde_json::Map::new();
    let mut jobs = serde_json::Map::new();
    for job in &ledger.jobs {
        let referenced = std::iter::once(&job.home).chain(&job.entry_points).chain(
            job.variants
                .iter()
                .flat_map(|variant| [&variant.symbol, &variant.anchor]),
        );
        for path in referenced {
            let entry = workspace.resolve(path).map_or(Json::Null, |symbol| {
                let definition = &workspace.definitions[&symbol];
                json!({
                    "symbol": symbol,
                    "package": definition.package,
                    "file": definition.file,
                    "line": definition.line,
                    "module": definition.module,
                    "documented": definition.documented,
                })
            });
            symbols.insert(path.clone(), entry);
        }
        let Ok(resolved) = home(job, workspace) else {
            jobs.insert(job.id.clone(), Json::Null);
            continue;
        };
        let found = matches(job, workspace, &resolved.package);
        jobs.insert(
            job.id.clone(),
            json!({
                "home": resolved.symbol,
                "home_package": resolved.package,
                "copies": copies(&found),
                "matches": found
                    .iter()
                    .map(|found| {
                        json!({
                            "symbol": found.symbol,
                            "package": found.package,
                            "file": found.file,
                            "line": found.line,
                            "reasons": found.reasons,
                            "in_home": found.in_home,
                            "variant": found.variant,
                        })
                    })
                    .collect::<Vec<_>>(),
            }),
        );
    }
    let variants = variant_symbols(ledger, workspace, "isomorphic");
    let grouped: BTreeSet<&str> = isomorphic_groups(workspace)
        .iter()
        .chain(&shim_groups(workspace))
        .flat_map(|group| {
            group
                .members
                .iter()
                .map(|&index| workspace.units[index].symbol.as_str())
        })
        .collect();
    json!({
        "symbols": symbols,
        "jobs": jobs,
        "grouped_variants": variants.iter().filter(|symbol| grouped.contains(symbol.as_str())).collect::<Vec<_>>(),
        "errors": workspace.errors,
    })
}

/// Every `--check` finding, one line each.
pub(crate) fn check_findings(workspace: &Workspace, ledger: &Ledger) -> Vec<String> {
    let mut findings: Vec<String> = workspace.errors.clone();
    let variants = variant_symbols(ledger, workspace, "isomorphic");
    for (label, groups) in [
        ("isomorphic bodies", isomorphic_groups(workspace)),
        ("isomorphic shims", shim_groups(workspace)),
    ] {
        for group in groups {
            if group_is_sanctioned(&group, workspace, &variants) {
                continue;
            }
            let members: Vec<String> = group
                .members
                .iter()
                .map(|&index| {
                    let unit = &workspace.units[index];
                    format!("{} ({}:{})", unit.symbol, unit.file, unit.line)
                })
                .collect();
            findings.push(format!(
                "{label} {}: {}",
                group.fingerprint,
                members.join(", ")
            ));
        }
    }
    for job in &ledger.jobs {
        match home(job, workspace) {
            Err(error) => findings.push(error),
            Ok(resolved) => {
                for found in matches(job, workspace, &resolved.package) {
                    if !found.in_home && !found.variant {
                        findings.push(format!(
                            "job `{}`: {} ({}:{}) outside home {}: {}",
                            job.id,
                            found.symbol,
                            found.file,
                            found.line,
                            resolved.package,
                            found.reasons.join(", ")
                        ));
                    }
                }
            }
        }
    }
    findings
}

/// The human `--census` listing for one job.
pub(crate) fn census_lines(job: &Job, workspace: &Workspace) -> Result<Vec<String>, String> {
    let resolved = home(job, workspace)?;
    let found = matches(job, workspace, &resolved.package);
    let mut lines = vec![format!(
        "job {}: home {} ({}), {}",
        job.id,
        resolved.symbol,
        resolved.package,
        if job.enforced {
            "enforced"
        } else {
            "not yet enforced"
        }
    )];
    for found in &found {
        let role = if found.in_home {
            "HOME   "
        } else if found.variant {
            "VARIANT"
        } else {
            "COPY   "
        };
        lines.push(format!(
            "  {role} {}:{} {} [{}]",
            found.file,
            found.line,
            found.symbol,
            found.reasons.join(", ")
        ));
    }
    lines.push(format!("copies={}", copies(&found)));
    Ok(lines)
}
