// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed view of `helpers-ledger.toml`.
//!
//! The file's header comment is the schema. Every key a row needs is required and
//! every key this reader does not know is refused: a misspelt `enforcd = true`
//! must fail loudly rather than leave a job silently unenforced.

use std::collections::BTreeMap;

use crate::toml::{self, Value};

/// Why a second implementation of a job may exist.
pub(crate) const CRITERIA: [&str; 4] = ["a", "b", "c", "layering"];

/// One sanctioned second implementation. Its `criterion` and `reason`
/// are validated here and checked against the tree by the engine.
#[derive(Clone, Debug)]
pub(crate) struct Variant {
    /// The variant's own symbol, `crate::path::item`.
    pub(crate) symbol: String,
    /// The repo-relative file that defines it.
    pub(crate) file: String,
    /// Which detector it is exempt from: `isomorphic` and `forbidden` are this
    /// census's; any other id names a rule of the gate that owns it.
    pub(crate) detector: String,
    /// The documented item that states why the variant exists.
    pub(crate) anchor: String,
}

/// What a job forbids outside its home crate.
#[derive(Clone, Debug, Default)]
pub(crate) struct Forbidden {
    /// Integer literals, any spelling; compared by value.
    pub(crate) constants: Vec<u128>,
    /// `body:…`, `shim:…` or `table:…` fingerprint ids.
    pub(crate) fingerprints: Vec<String>,
    /// Regular expressions over function names.
    pub(crate) names: Vec<regex::Regex>,
}

/// One job: a piece of functionality with exactly one home.
#[derive(Clone, Debug)]
pub(crate) struct Job {
    /// The row id `--census` and `--count` take.
    pub(crate) id: String,
    /// The single implementation, `crate::path::item`.
    pub(crate) home: String,
    /// Further public entry points of the same implementation.
    pub(crate) entry_points: Vec<String>,
    /// Whether a forbidden match outside the home fails the gate today.
    pub(crate) enforced: bool,
    /// What is forbidden outside the home.
    pub(crate) forbidden: Forbidden,
    /// Sanctioned second implementations.
    pub(crate) variants: Vec<Variant>,
}

/// The whole ledger.
#[derive(Clone, Debug)]
pub(crate) struct Ledger {
    /// Rows in file order.
    pub(crate) jobs: Vec<Job>,
    /// The parsed document, for the engine's cross-check.
    pub(crate) document: BTreeMap<String, Value>,
}

impl Ledger {
    /// The job with this id.
    pub(crate) fn job(&self, id: &str) -> Option<&Job> {
        self.jobs.iter().find(|job| job.id == id)
    }
}

const JOB_KEYS: [&str; 12] = [
    "id",
    "summary",
    "home",
    "entry_points",
    "spec",
    "vectors",
    "bench",
    "sites",
    "replaces_external",
    "enforced",
    "forbidden",
    "variant",
];
const JOB_OPTIONAL: [&str; 1] = ["variant"];
const FORBIDDEN_KEYS: [&str; 3] = ["constants", "fingerprints", "names"];
const VARIANT_KEYS: [&str; 6] = [
    "symbol",
    "file",
    "detector",
    "criterion",
    "anchor",
    "reason",
];

/// Parse the ledger text.
pub(crate) fn parse(text: &str) -> Result<Ledger, String> {
    let document = toml::parse(text).map_err(|error| format!("helpers-ledger.toml {error}"))?;
    for key in document.keys() {
        if key != "job" {
            return Err(format!(
                "helpers-ledger.toml: unknown top-level key `{key}`"
            ));
        }
    }
    let rows = match document.get("job") {
        None => Vec::new(),
        Some(Value::Array(rows)) => rows.clone(),
        Some(_) => return Err("helpers-ledger.toml: `job` must be an array of tables".to_owned()),
    };
    let mut jobs = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let Value::Table(row) = row else {
            return Err(format!("job #{index} is not a table"));
        };
        let job =
            job(row).map_err(|error| format!("helpers-ledger.toml job #{}: {error}", index + 1))?;
        if jobs.iter().any(|seen: &Job| seen.id == job.id) {
            return Err(format!(
                "helpers-ledger.toml: job id `{}` is used twice",
                job.id
            ));
        }
        jobs.push(job);
    }
    Ok(Ledger { jobs, document })
}

fn check_keys(
    table: &BTreeMap<String, Value>,
    known: &[&str],
    optional: &[&str],
    what: &str,
) -> Result<(), String> {
    for key in table.keys() {
        if !known.contains(&key.as_str()) {
            return Err(format!("unknown {what} key `{key}`"));
        }
    }
    for key in known {
        if !optional.contains(key) && !table.contains_key(*key) {
            return Err(format!("{what} is missing required key `{key}`"));
        }
    }
    Ok(())
}

fn string(table: &BTreeMap<String, Value>, key: &str) -> Result<String, String> {
    match table.get(key) {
        Some(Value::Str(text)) if !text.trim().is_empty() => Ok(text.clone()),
        Some(Value::Str(_)) => Err(format!("`{key}` must not be empty")),
        _ => Err(format!("`{key}` must be a string")),
    }
}

fn strings(table: &BTreeMap<String, Value>, key: &str) -> Result<Vec<String>, String> {
    let Some(Value::Array(items)) = table.get(key) else {
        return Err(format!("`{key}` must be an array of strings"));
    };
    items
        .iter()
        .map(|item| match item {
            Value::Str(text) if !text.trim().is_empty() => Ok(text.clone()),
            _ => Err(format!("`{key}` must hold non-empty strings only")),
        })
        .collect()
}

fn job(row: &BTreeMap<String, Value>) -> Result<Job, String> {
    check_keys(row, &JOB_KEYS, &JOB_OPTIONAL, "job")?;
    let id = string(row, "id")?;
    for key in ["summary", "spec"] {
        string(row, key).map_err(|error| format!("{id}: {error}"))?;
    }
    for key in ["vectors", "bench", "sites", "replaces_external"] {
        strings(row, key).map_err(|error| format!("{id}: {error}"))?;
    }
    let Some(Value::Bool(enforced)) = row.get("enforced") else {
        return Err(format!("{id}: `enforced` must be a boolean"));
    };
    let Some(Value::Table(forbidden)) = row.get("forbidden") else {
        return Err(format!("{id}: `forbidden` must be a [job.forbidden] table"));
    };
    let variants = match row.get("variant") {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::Table(table) => variant(table),
                _ => Err("a variant must be a [[job.variant]] table".to_owned()),
            })
            .collect::<Result<_, _>>()
            .map_err(|error| format!("{id}: {error}"))?,
        Some(_) => return Err(format!("{id}: `variant` must be an array of tables")),
    };
    Ok(Job {
        home: string(row, "home")?,
        entry_points: strings(row, "entry_points")?,
        enforced: *enforced,
        forbidden: forbidden_table(forbidden).map_err(|error| format!("{id}: {error}"))?,
        variants,
        id,
    })
}

fn forbidden_table(table: &BTreeMap<String, Value>) -> Result<Forbidden, String> {
    check_keys(table, &FORBIDDEN_KEYS, &[], "forbidden")?;
    let constants = strings(table, "constants")?
        .iter()
        .map(|spelling| {
            crate::normalize::integer_value(spelling)
                .ok_or_else(|| format!("forbidden constant `{spelling}` is not an integer literal"))
        })
        .collect::<Result<_, _>>()?;
    let fingerprints = strings(table, "fingerprints")?;
    for id in &fingerprints {
        if !crate::normalize::is_fingerprint_id(id) {
            return Err(format!(
                "forbidden fingerprint `{id}` is not a `body:`, `shim:`, `table:` or `rule:` id"
            ));
        }
    }
    let names = strings(table, "names")?
        .iter()
        .map(|pattern| {
            regex::Regex::new(pattern)
                .map_err(|error| format!("forbidden name `{pattern}`: {error}"))
        })
        .collect::<Result<_, _>>()?;
    Ok(Forbidden {
        constants,
        fingerprints,
        names,
    })
}

fn variant(table: &BTreeMap<String, Value>) -> Result<Variant, String> {
    check_keys(table, &VARIANT_KEYS, &[], "variant")?;
    let criterion = string(table, "criterion")?;
    if !CRITERIA.contains(&criterion.as_str()) {
        return Err(format!(
            "variant criterion `{criterion}` is not one of {}",
            CRITERIA.join(", ")
        ));
    }
    string(table, "reason")?;
    string(table, "file")?;
    Ok(Variant {
        symbol: string(table, "symbol")?,
        file: string(table, "file")?,
        detector: string(table, "detector")?,
        anchor: string(table, "anchor")?,
    })
}

#[cfg(test)]
mod tests {
    use super::parse;

    const ROW: &str = "[[job]]\nid = \"j\"\nsummary = \"s\"\nhome = \"c::f\"\nentry_points = []\nspec = \"x\"\nvectors = []\nbench = []\nsites = []\nreplaces_external = []\nenforced = true\n[job.forbidden]\nconstants = [\"0xcbf2_9ce4_8422_2325\"]\nfingerprints = [\"table:hex-lower\"]\nnames = ['^f$']\n";

    #[test]
    fn a_complete_row_is_read() {
        let ledger = parse(ROW).expect("a complete row");
        let job = ledger.job("j").expect("the row");
        assert!(job.enforced);
        assert_eq!(job.forbidden.constants, vec![14_695_981_039_346_656_037]);
    }

    #[test]
    fn a_missing_or_unknown_key_is_refused() {
        let missing = ROW.replace("enforced = true\n", "");
        assert!(parse(&missing).unwrap_err().contains("enforced"));
        let unknown = ROW.replace("enforced = true\n", "enforced = true\nenforcd = true\n");
        assert!(parse(&unknown).unwrap_err().contains("enforcd"));
    }

    #[test]
    fn a_variant_needs_a_known_criterion() {
        let variant = "[[job.variant]]\nsymbol = \"c::g\"\nfile = \"f.rs\"\ndetector = \"isomorphic\"\ncriterion = \"CRITERION\"\nanchor = \"c::g\"\nreason = \"r\"\n";
        let refused = format!("{ROW}{}", variant.replace("CRITERION", "d"));
        assert!(parse(&refused).unwrap_err().contains("criterion"));
        let accepted = format!("{ROW}{}", variant.replace("CRITERION", "layering"));
        assert_eq!(
            parse(&accepted).expect("layering").jobs[0].variants.len(),
            1
        );
    }
}
