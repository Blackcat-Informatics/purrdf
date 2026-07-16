// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::super::util::canonical_json_bounded;
use super::super::{ProjectionError, ProjectionLimits, ProjectionPackage};
use super::{LpgConfig, LpgGraph};

const PROFILE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CarrierManifest {
    profile: String,
    profile_version: u32,
    lpg_schema_version: u32,
}

pub(super) fn write_manifest(
    profile: &str,
    graph: &LpgGraph,
    config: &LpgConfig,
) -> Result<Vec<u8>, ProjectionError> {
    canonical_json_bounded(
        &CarrierManifest {
            profile: profile.to_owned(),
            profile_version: PROFILE_VERSION,
            lpg_schema_version: graph.schema_version,
        },
        config.limits(),
        "LPG carrier manifest",
    )
}

pub(super) fn read_manifest(
    bytes: &[u8],
    profile: &str,
    config: &LpgConfig,
    path: &str,
) -> Result<u32, ProjectionError> {
    let manifest: CarrierManifest = parse_json(bytes, config, "LPG carrier manifest", path)?;
    if manifest.profile != profile || manifest.profile_version != PROFILE_VERSION {
        return Err(ProjectionError::integrity(format!(
            "manifest identifies profile {:?} version {}; expected {profile:?} version {PROFILE_VERSION}",
            manifest.profile, manifest.profile_version
        ))
        .at_path(path));
    }
    Ok(manifest.lpg_schema_version)
}

pub(super) fn json_string<T: Serialize>(
    value: &T,
    config: &LpgConfig,
    description: &str,
) -> Result<String, ProjectionError> {
    String::from_utf8(canonical_json_bounded(value, config.limits(), description)?).map_err(
        |error| ProjectionError::integrity(format!("JSON encoder emitted non-UTF-8: {error}")),
    )
}

pub(super) fn parse_json<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
    config: &LpgConfig,
    description: &str,
    path: &str,
) -> Result<T, ProjectionError> {
    if bytes.len() > config.limits().max_artifact_bytes() {
        return Err(ProjectionError::limit(format!(
            "{description} exceeds the per-artifact byte limit"
        ))
        .at_path(path));
    }
    let value: T = serde_json::from_slice(bytes).map_err(|error| {
        ProjectionError::syntax(format!("parse {description}: {error}")).at_path(path)
    })?;
    if canonical_json_bounded(&value, config.limits(), description)? != bytes {
        return Err(ProjectionError::syntax(format!(
            "{description} is not in canonical PurRDF form"
        ))
        .at_path(path));
    }
    Ok(value)
}

pub(super) fn required_artifact<'a>(
    package: &'a ProjectionPackage,
    path: &str,
) -> Result<&'a [u8], ProjectionError> {
    package
        .get(path)
        .ok_or_else(|| ProjectionError::package("required artifact is missing").at_path(path))
}

pub(super) fn validate_package_bounds(
    package: &ProjectionPackage,
    limits: ProjectionLimits,
) -> Result<(), ProjectionError> {
    if package.len() > limits.max_artifacts() {
        return Err(ProjectionError::limit(format!(
            "package has {} artifacts; reader limit is {}",
            package.len(),
            limits.max_artifacts()
        )));
    }
    if package.total_bytes() > limits.max_total_bytes()
        || package.archive_bytes() > limits.max_archive_bytes()
    {
        return Err(ProjectionError::limit(
            "package exceeds the configured total or archive byte limit",
        ));
    }
    for (path, bytes) in package.artifacts() {
        if bytes.len() > limits.max_artifact_bytes() {
            return Err(ProjectionError::limit(format!(
                "artifact is {} bytes; reader limit is {}",
                bytes.len(),
                limits.max_artifact_bytes()
            ))
            .at_path(path));
        }
    }
    Ok(())
}

pub(super) fn require_canonical_package(
    actual: &ProjectionPackage,
    canonical: &ProjectionPackage,
    profile: &str,
) -> Result<(), ProjectionError> {
    if !actual.artifacts().eq(canonical.artifacts()) {
        return Err(ProjectionError::syntax(format!(
            "{profile} package is valid but not in canonical PurRDF form"
        )));
    }
    Ok(())
}

pub(super) struct BoundedText {
    bytes: Vec<u8>,
    limit: usize,
    description: &'static str,
    path: &'static str,
}

impl BoundedText {
    pub(super) fn new(
        limits: ProjectionLimits,
        description: &'static str,
        path: &'static str,
    ) -> Self {
        Self {
            bytes: Vec::new(),
            limit: limits.max_artifact_bytes(),
            description,
            path,
        }
    }

    pub(super) fn push(&mut self, value: &str) -> Result<(), ProjectionError> {
        self.ensure_additional(value.len())?;
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    pub(super) fn push_hex(&mut self, value: &[u8]) -> Result<(), ProjectionError> {
        let added = value.len().checked_mul(2).ok_or_else(|| {
            ProjectionError::limit(format!("{} byte count overflow", self.description))
        })?;
        self.ensure_additional(added)?;
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in value {
            self.bytes.push(HEX[usize::from(byte >> 4)]);
            self.bytes.push(HEX[usize::from(byte & 0x0f)]);
        }
        Ok(())
    }

    fn ensure_additional(&self, added: usize) -> Result<(), ProjectionError> {
        let length = self.bytes.len().checked_add(added).ok_or_else(|| {
            ProjectionError::limit(format!("{} byte count overflow", self.description))
        })?;
        if length > self.limit {
            return Err(ProjectionError::limit(format!(
                "{} exceeds the {}-byte artifact limit",
                self.description, self.limit
            ))
            .at_path(self.path));
        }
        Ok(())
    }

    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(super) fn hex_decode(
    value: &str,
    description: &str,
    path: &str,
) -> Result<Vec<u8>, ProjectionError> {
    if !value.len().is_multiple_of(2) {
        return Err(ProjectionError::syntax(format!(
            "{description} lowercase-hex payload has odd length"
        ))
        .at_path(path));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let high = hex_nibble(pair[0]).ok_or_else(|| {
            ProjectionError::syntax(format!("{description} contains a non-lowercase-hex digit"))
                .at_path(path)
        })?;
        let low = hex_nibble(pair[1]).ok_or_else(|| {
            ProjectionError::syntax(format!("{description} contains a non-lowercase-hex digit"))
                .at_path(path)
        })?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
