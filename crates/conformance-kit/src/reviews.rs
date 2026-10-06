// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent oracle admission bound to actual payload and derivation bytes.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_lex::json::{
    self,
    record::{DecodeError, FromJson as _},
};

use crate::{
    GradeError,
    inventory::{Catalog, Expected, Inventory},
};

/// Digest of one normative input or independently written derivation document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// Ordinary root-relative corpus path.
    pub path: String,
    /// Canonical lowercase BLAKE3-256 of the actual bytes.
    pub blake3: String,
}
purrdf_lex::json_record!(Artifact as "reviewed artifact" {
    "path" => path: required,
    "blake3" => blake3: required,
});

/// One independently approved case, with no observed engine answer as input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    /// Exact inventory case identity.
    pub id: String,
    /// Only accepted reviews admit a normative oracle.
    pub status: String,
    /// Explicit independent reviewer identity.
    pub reviewer: String,
    /// Root-relative independent derivation document; its digest is mandatory.
    pub derivation: String,
    /// Exact normative input and derivation artifact set for this case.
    pub payloads: Vec<Artifact>,
}
purrdf_lex::json_record!(Case as "independent case review" {
    "id" => id: required,
    "status" => status: required,
    "reviewer" => reviewer: required,
    "derivation" => derivation: required,
    "payloads" => payloads: required,
});

/// The single review registry referenced by a corpus catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// Exact portable schema identity.
    pub contract_version: u32,
    /// Ownership notice for the registry.
    pub copyright: String,
    /// Explicit license of the review record.
    pub license: String,
    /// Exactly one accepted record for every catalogued case.
    pub cases: Vec<Case>,
}
purrdf_lex::json_record!(Index as "independent review registry" {
    "contractVersion" => contract_version: required,
    "copyright" => copyright: required,
    "license" => license: required,
    "cases" => cases: required,
});

/// Actual independently acquired bytes, before parsing or engine execution.
#[derive(Debug, Clone, Copy)]
pub struct Payload<'a> {
    /// Root-relative artifact name.
    pub path: &'a str,
    /// Unmodified acquired bytes.
    pub bytes: &'a [u8],
}

/// Strictly decode a registry. Admission against actual inputs is separate.
///
/// # Errors
/// Refuses unknown record fields, revisions or malformed JSON.
pub fn decode(bytes: &[u8]) -> Result<Index, DecodeError> {
    let index = Index::from_json(
        &json::read_slice(bytes, json::Limits::DEFAULT).map_err(DecodeError::custom)?,
    )?;
    if index.contract_version != 1
        || index.cases.is_empty()
        || index.copyright.trim().is_empty()
        || index.license.trim().is_empty()
    {
        return Err(DecodeError::custom(
            "unknown or empty independent review registry",
        ));
    }
    Ok(index)
}

/// Admit every independent review against the catalog, inventories and actual
/// input bytes. Expected artifact sets derive from inventory paths and manifest
/// roots, not from registry claims or produced outputs. Derivation documents are
/// also hashed. Catalogs, inventories and the registry cannot hash themselves.
///
/// # Errors
/// Refuses missing/duplicate cases, unaccepted reviews, changed/missing bytes,
/// circular provenance or an incomplete/differently associated artifact set.
pub fn admit(
    catalog: &Catalog,
    inventories: &[Inventory],
    index: &Index,
    payloads: &[Payload<'_>],
) -> Result<(), GradeError> {
    let registry_path = catalog
        .review_index
        .as_deref()
        .ok_or_else(|| malformed("catalog does not declare an independent review registry"))?;
    if normalized_path(".", registry_path)? != registry_path
        || index.contract_version != 1
        || index.copyright.trim().is_empty()
        || index.license.trim().is_empty()
        || catalog.suites.len() != inventories.len()
    {
        return Err(malformed(
            "registry identity or inventory count disagrees with the catalog",
        ));
    }
    let mut actual = BTreeMap::new();
    for payload in payloads {
        if normalized_path(".", payload.path)? != payload.path
            || actual.insert(payload.path, payload.bytes).is_some()
        {
            return Err(malformed(
                "actual artifact paths are noncanonical or repeated",
            ));
        }
    }
    let mut reviews = BTreeMap::new();
    for review in &index.cases {
        if review.id.is_empty()
            || review.status != "accepted"
            || review.reviewer.trim().is_empty()
            || reviews.insert(review.id.as_str(), review).is_some()
        {
            return Err(malformed(
                "independent review identity, reviewer or accepted status is absent/repeated",
            ));
        }
    }
    let mut forbidden = BTreeSet::from(["catalog.json".to_owned(), registry_path.to_owned()]);
    forbidden.extend(catalog.suites.iter().map(|suite| suite.inventory.clone()));
    let mut ids = BTreeSet::new();
    for (suite, inventory) in catalog.suites.iter().zip(inventories) {
        if inventory.cases.len() != suite.case_count {
            return Err(malformed(
                "review inventory case count differs from the catalog",
            ));
        }
        for case in &inventory.cases {
            if !ids.insert(case.id.as_str()) {
                return Err(malformed("case identities repeat across inventories"));
            }
            let review = reviews.remove(case.id.as_str()).ok_or_else(|| {
                malformed("a catalogued case lacks an independent accepted review")
            })?;
            let mut required: BTreeSet<_> = suite
                .manifests
                .iter()
                .map(|path| normalized_path(".", path))
                .collect::<Result<_, _>>()?;
            for path in case
                .manifest
                .iter()
                .chain(case.query.iter())
                .chain(case.shapes.iter())
                .map(String::as_str)
                .chain(case.data.iter().map(|input| input.path.as_str()))
                .chain(
                    case.services
                        .iter()
                        .map(|service| service.data.path.as_str()),
                )
            {
                required.insert(normalized_path(&suite.directory, path)?);
            }
            match &case.expected {
                Expected::File(path) => {
                    required.insert(normalized_path(&suite.directory, path)?);
                }
                Expected::Outcome(spec) => {
                    if let Some(path) = &spec.path {
                        required.insert(normalized_path(&suite.directory, path)?);
                    }
                }
                Expected::None => {}
            }
            let derivation = normalized_path(".", &review.derivation)?;
            if derivation != review.derivation || required.contains(&derivation) {
                return Err(malformed(
                    "review derivation must be a separate canonical document",
                ));
            }
            required.insert(derivation);
            let declared: BTreeSet<_> = review
                .payloads
                .iter()
                .map(|artifact| artifact.path.clone())
                .collect();
            if declared.len() != review.payloads.len()
                || declared != required
                || declared.iter().any(|path| forbidden.contains(path))
            {
                return Err(malformed(
                    "review artifact set is incomplete, contradictory or circular",
                ));
            }
            for artifact in &review.payloads {
                let expected = purrdf_hash::hex::Digest32::from_hex(&artifact.blake3)
                    .ok_or_else(|| malformed("review digest is not canonical BLAKE3-256"))?;
                let bytes = actual
                    .get(artifact.path.as_str())
                    .ok_or_else(|| malformed("reviewed artifact bytes were not acquired"))?;
                let actual = purrdf_hash::blake3::hash(bytes);
                if expected.as_bytes() != actual.as_bytes() {
                    return Err(malformed("reviewed input or derivation bytes have changed"));
                }
            }
        }
    }
    if ids.len() != catalog.case_count || !reviews.is_empty() {
        return Err(malformed(
            "review registry and complete catalog case closure disagree",
        ));
    }
    Ok(())
}

fn normalized_path(directory: &str, path: &str) -> Result<String, GradeError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', '\0'])
        || directory.starts_with('/')
        || directory.contains(['\\', '\0'])
    {
        return Err(malformed(
            "review artifact is not an ordinary relative corpus path",
        ));
    }
    let mut segments = Vec::new();
    for segment in directory.split('/').chain(path.split('/')) {
        match segment {
            "." => {}
            "" | ".." => {
                return Err(malformed(
                    "review artifact path contains empty/traversal segments",
                ));
            }
            _ => segments.push(segment),
        }
    }
    if segments.is_empty() {
        return Err(malformed("review artifact path is empty"));
    }
    Ok(segments.join("/"))
}

fn malformed(message: &str) -> GradeError {
    GradeError::Malformed(message.to_owned())
}
