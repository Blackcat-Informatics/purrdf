// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Grouping, ledger matching and the reports built from them.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_lex::json::{Object, Value as Json};

use crate::ledger::{Distinct, GroupKind, Job, Ledger};
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

/// A group's member symbols, sorted: the set a [`Distinct`] row must equal.
fn member_symbols<'w>(group: &Group, workspace: &'w Workspace) -> Vec<&'w str> {
    let mut symbols: Vec<&str> = group
        .members
        .iter()
        .map(|&index| workspace.units[index].symbol.as_str())
        .collect();
    symbols.sort_unstable();
    symbols
}

/// The `[[distinct]]` row whose kind is `kind` and whose members are exactly the
/// group's, if there is one.
fn distinct_row(ledger: &Ledger, kind: GroupKind, symbols: &[&str]) -> Option<usize> {
    ledger.distinct.iter().position(|row| {
        row.kind == kind
            && row
                .members
                .iter()
                .map(String::as_str)
                .eq(symbols.iter().copied())
    })
}

/// Every kind of group, with the label `--check` reports it under.
fn every_group(workspace: &Workspace) -> [(GroupKind, &'static str, Vec<Group>); 2] {
    [
        (
            GroupKind::Body,
            "isomorphic bodies",
            isomorphic_groups(workspace),
        ),
        (GroupKind::Shim, "isomorphic shims", shim_groups(workspace)),
    ]
}

/// A `[[distinct]]` row's own failures: a member that is no longer defined, a
/// member set that is no current group (both STALE), and an anchor that does not
/// resolve or carries no documentation.
fn distinct_row_findings(row: &Distinct, matched: bool, workspace: &Workspace) -> Vec<String> {
    let mut findings = Vec::new();
    let label = row.label();
    let missing: Vec<&str> = row
        .members
        .iter()
        .filter(|member| !workspace.definitions.contains_key(member.as_str()))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        findings.push(format!(
            "{label}: STALE, member {} no longer exists; remove the row",
            missing.join(", ")
        ));
    } else if !matched {
        findings.push(format!(
            "{label}: STALE, no current {} group has exactly these members; remove or correct the row",
            row.kind.as_str()
        ));
    }
    match workspace.resolve(&row.anchor) {
        None => findings.push(format!("{label}: anchor `{}` does not resolve", row.anchor)),
        Some(symbol) if !workspace.definitions[&symbol].documented => findings.push(format!(
            "{label}: anchor `{}` carries no documentation",
            row.anchor
        )),
        Some(_) => {}
    }
    findings
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
                in_home: unit.package == home_package && !second_home(job, workspace, unit),
                variant: variants.contains(&unit.symbol),
            });
        }
    }
    if job
        .forbidden
        .fingerprints
        .iter()
        .any(|id| id == crate::rules::VOCABULARY_LITERAL)
    {
        found.extend(vocabulary_matches(job, workspace, &variants));
    }
    if job
        .forbidden
        .fingerprints
        .iter()
        .any(|id| id == crate::rules::HOME_LITERAL)
    {
        found.extend(home_literal_matches(job, workspace, &variants));
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

/// The vocabulary modules of a job: its home and entry points, resolved.
fn vocabulary_modules(job: &Job, workspace: &Workspace) -> Vec<String> {
    let mut modules: Vec<String> = std::iter::once(&job.home)
        .chain(&job.entry_points)
        .filter_map(|path| workspace.resolve(path))
        .collect();
    modules.sort();
    modules.dedup();
    modules
}

fn under(symbol: &str, modules: &[String]) -> bool {
    modules.iter().any(|module| {
        symbol
            .strip_prefix(module.as_str())
            .is_some_and(|rest| rest.starts_with("::"))
    })
}

/// Every `rule:vocabulary-literal` match: a string literal outside the job's
/// vocabulary modules that equals or starts with a namespace those modules
/// declare. A job whose modules declare no namespace is itself a match, so
/// the rule can never pass by reading nothing.
fn vocabulary_matches(job: &Job, workspace: &Workspace, variants: &BTreeSet<String>) -> Vec<Match> {
    let modules = vocabulary_modules(job, workspace);
    let mut namespaces: Vec<String> = workspace
        .units
        .iter()
        .filter(|unit| {
            unit.kind == UnitKind::Constant
                && crate::rules::is_namespace_name(&unit.name)
                && under(&unit.symbol, &modules)
        })
        .filter_map(|unit| match unit.strings.as_slice() {
            [(namespace, _)] if !namespace.is_empty() => Some(namespace.clone()),
            _ => None,
        })
        .collect();
    namespaces.sort();
    namespaces.dedup();
    if namespaces.is_empty() {
        return vec![Match {
            symbol: job.home.clone(),
            package: String::new(),
            file: String::new(),
            line: 0,
            reasons: vec![format!(
                "{}: the modules {} declare no namespace constant (`NS` or `*_NS`)",
                crate::rules::VOCABULARY_LITERAL,
                if modules.is_empty() {
                    "(none resolve)".to_owned()
                } else {
                    modules.join(", ")
                }
            )],
            in_home: false,
            variant: false,
        }];
    }
    let mut found = Vec::new();
    for unit in &workspace.units {
        if under(&unit.symbol, &modules) {
            continue;
        }
        for (literal, line) in &unit.strings {
            if let Some(namespace) = crate::rules::vocabulary_namespace(literal, &namespaces) {
                found.push(Match {
                    symbol: unit.symbol.clone(),
                    package: unit.package.clone(),
                    file: unit.file.clone(),
                    line: *line,
                    reasons: vec![format!(
                        "{}: \"{literal}\" spells out a term of <{namespace}>",
                        crate::rules::VOCABULARY_LITERAL
                    )],
                    in_home: false,
                    variant: variants.contains(&unit.symbol),
                });
            }
        }
    }
    found
}

/// Every `rule:home-literal` match: a string literal outside the job's home
/// item equal to one the home item's bodies hold. A home that holds no literal
/// is itself a match, so the rule can never pass by reading nothing.
fn home_literal_matches(
    job: &Job,
    workspace: &Workspace,
    variants: &BTreeSet<String>,
) -> Vec<Match> {
    let home = workspace.resolve(&job.home).into_iter().collect::<Vec<_>>();
    let is_home = |symbol: &str| home.iter().any(|home| symbol == home) || under(symbol, &home);
    let mut tokens: Vec<&str> = workspace
        .units
        .iter()
        .filter(|unit| is_home(&unit.symbol))
        .flat_map(|unit| unit.strings.iter().map(|(text, _)| text.as_str()))
        .filter(|text| !text.is_empty())
        .collect();
    tokens.sort_unstable();
    tokens.dedup();
    if tokens.is_empty() {
        return vec![Match {
            symbol: job.home.clone(),
            package: String::new(),
            file: String::new(),
            line: 0,
            reasons: vec![format!(
                "{}: the home `{}` holds no string literal",
                crate::rules::HOME_LITERAL,
                job.home
            )],
            in_home: false,
            variant: false,
        }];
    }
    let mut found = Vec::new();
    for unit in &workspace.units {
        if is_home(&unit.symbol) {
            continue;
        }
        for (literal, line) in &unit.strings {
            if tokens.binary_search(&literal.as_str()).is_ok() {
                found.push(Match {
                    symbol: unit.symbol.clone(),
                    package: unit.package.clone(),
                    file: unit.file.clone(),
                    line: *line,
                    reasons: vec![format!(
                        "{}: \"{literal}\" is a token `{}` spells",
                        crate::rules::HOME_LITERAL,
                        job.home
                    )],
                    in_home: false,
                    variant: variants.contains(&unit.symbol),
                });
            }
        }
    }
    found
}

/// Whether `unit` matches the job by name yet is neither the home nor one of its
/// entry points, inside the home crate: a second home. A constant is always a
/// copy there (a bound redeclared beside its home is a second bound); a function
/// is one only for a job that lists [`crate::rules::SOLE_NAME`], since a home
/// crate's own sibling helpers may legitimately share a job's name pattern.
fn second_home(job: &Job, workspace: &Workspace, unit: &Unit) -> bool {
    let strict = match unit.kind {
        UnitKind::Constant => true,
        UnitKind::Function => job
            .forbidden
            .fingerprints
            .iter()
            .any(|id| id == crate::rules::SOLE_NAME),
    };
    strict
        && job
            .forbidden
            .names
            .iter()
            .any(|pattern| pattern.is_match(&unit.name))
        && std::iter::once(&job.home)
            .chain(&job.entry_points)
            .all(|path| workspace.resolve(path).as_deref() != Some(unit.symbol.as_str()))
}

fn unit_reasons(job: &Job, unit: &Unit) -> Vec<String> {
    let mut reasons = Vec::new();
    if matches!(unit.kind, UnitKind::Function | UnitKind::Constant) {
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
    Json::from(
        Object::new()
            .with("symbol", &unit.symbol)
            .with("package", &unit.package)
            .with("file", &unit.file)
            .with("line", unit.line),
    )
}

fn group_json(group: &Group, workspace: &Workspace, variants: &BTreeSet<String>) -> Json {
    Json::from(
        Object::new()
            .with("fingerprint", &group.fingerprint)
            .with("tokens", group.tokens)
            .with(
                "sanctioned",
                group_is_sanctioned(group, workspace, variants),
            )
            .with(
                "members",
                group
                    .members
                    .iter()
                    .map(|&index| unit_json(&workspace.units[index]))
                    .collect::<Json>(),
            ),
    )
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
            entry["tables"] = unit.tables.iter().copied().collect();
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
            Json::from(
                Object::new()
                    .with("value", format!("{value:#x}"))
                    .with("packages", packages.len())
                    .with(
                        "units",
                        packages
                            .values()
                            .flatten()
                            .map(|unit| unit_json(unit))
                            .collect::<Json>(),
                    ),
            )
        })
        .collect();
    let mut document = Json::from(
        Object::new()
            .with("min_tokens", MIN_TOKENS)
            .with("units", workspace.units.len())
            .with(
                "isomorphic_groups",
                groups
                    .iter()
                    .map(|group| group_json(group, workspace, &variants))
                    .collect::<Json>(),
            )
            .with(
                "shim_groups",
                shims
                    .iter()
                    .map(|group| group_json(group, workspace, &variants))
                    .collect::<Json>(),
            )
            .with("hex_tables", tables)
            .with("shared_constants", shared),
    );
    // Members by name, at every level: the document's published layout.
    document.sort_keys();
    document
}

/// The document `scripts/check-shared-helpers.py` reads.
pub(crate) fn index(workspace: &Workspace, ledger: &Ledger) -> Json {
    let mut symbols = Object::new();
    let mut jobs = Object::new();
    for job in &ledger.jobs {
        let referenced = std::iter::once(&job.home).chain(&job.entry_points).chain(
            job.variants
                .iter()
                .flat_map(|variant| [&variant.symbol, &variant.anchor]),
        );
        for path in referenced {
            let entry = workspace.resolve(path).map_or(Json::Null, |symbol| {
                let definition = &workspace.definitions[&symbol];
                Json::from(
                    Object::new()
                        .with("symbol", symbol)
                        .with("package", &definition.package)
                        .with("file", &definition.file)
                        .with("line", definition.line)
                        .with("module", &definition.module)
                        .with("documented", definition.documented),
                )
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
            Object::new()
                .with("home", resolved.symbol)
                .with("home_package", resolved.package)
                .with("copies", copies(&found))
                .with(
                    "matches",
                    found
                        .iter()
                        .map(|found| {
                            Object::new()
                                .with("symbol", &found.symbol)
                                .with("package", &found.package)
                                .with("file", &found.file)
                                .with("line", found.line)
                                .with("reasons", found.reasons.as_slice())
                                .with("in_home", found.in_home)
                                .with("variant", found.variant)
                        })
                        .collect::<Json>(),
                ),
        );
    }
    for row in &ledger.distinct {
        for path in row.members.iter().chain(std::iter::once(&row.anchor)) {
            let entry = workspace.resolve(path).map_or(Json::Null, |symbol| {
                let definition = &workspace.definitions[&symbol];
                Json::from(
                    Object::new()
                        .with("symbol", symbol)
                        .with("package", &definition.package)
                        .with("file", &definition.file)
                        .with("line", definition.line)
                        .with("module", &definition.module)
                        .with("documented", definition.documented),
                )
            });
            symbols.insert(path.clone(), entry);
        }
    }
    let variants = variant_symbols(ledger, workspace, "isomorphic");
    let groups: Vec<Json> = every_group(workspace)
        .iter()
        .flat_map(|(kind, _, groups)| {
            groups.iter().map(|group| {
                Json::from(
                    Object::new()
                        .with("kind", kind.as_str())
                        .with("fingerprint", &group.fingerprint)
                        .with("members", member_symbols(group, workspace))
                        .with(
                            "variant_sanctioned",
                            group_is_sanctioned(group, workspace, &variants),
                        ),
                )
            })
        })
        .collect();
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
    let mut document = Json::from(
        Object::new()
            .with("symbols", symbols)
            .with("jobs", jobs)
            .with("groups", groups)
            .with(
                "grouped_variants",
                variants
                    .iter()
                    .filter(|symbol| grouped.contains(symbol.as_str()))
                    .collect::<Json>(),
            )
            .with("errors", workspace.errors.as_slice()),
    );
    // Members by name, at every level: the document's published layout.
    document.sort_keys();
    document
}

/// Every `--check` finding, one line each.
pub(crate) fn check_findings(workspace: &Workspace, ledger: &Ledger) -> Vec<String> {
    let mut findings: Vec<String> = workspace.errors.clone();
    let variants = variant_symbols(ledger, workspace, "isomorphic");
    let mut matched = vec![false; ledger.distinct.len()];
    for (kind, label, groups) in every_group(workspace) {
        for group in groups {
            if let Some(row) = distinct_row(ledger, kind, &member_symbols(&group, workspace)) {
                matched[row] = true;
                continue;
            }
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
    for (row, matched) in ledger.distinct.iter().zip(matched) {
        findings.extend(distinct_row_findings(row, matched, workspace));
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
