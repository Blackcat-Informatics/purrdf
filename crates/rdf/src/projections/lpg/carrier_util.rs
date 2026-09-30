// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_lex::json::{Object, Value};

use super::super::json_codec::{Fields, FromJson, JsonError, ToJson};
use super::super::util::canonical_json_bounded;
use super::super::{ProjectionError, ProjectionLimits, ProjectionPackage};
use super::{LpgConfig, LpgGraph};

const PROFILE_VERSION: u32 = 1;

/// The versioned manifest every LPG carrier package writes beside its artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CarrierManifest {
    pub(super) profile: String,
    pub(super) profile_version: u32,
    pub(super) lpg_schema_version: u32,
}

impl FromJson for CarrierManifest {
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        let mut fields = Fields::new(value, "struct CarrierManifest")?;
        let manifest = Self {
            profile: fields.required("profile")?,
            profile_version: fields.required("profile_version")?,
            lpg_schema_version: fields.required("lpg_schema_version")?,
        };
        fields.deny_unknown()?;
        Ok(manifest)
    }
}

impl ToJson for CarrierManifest {
    fn to_json(&self) -> Value {
        Value::Object(
            Object::new()
                .with("profile", self.profile.as_str())
                .with("profile_version", self.profile_version)
                .with("lpg_schema_version", self.lpg_schema_version),
        )
    }
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

pub(super) fn json_string<T: ToJson + ?Sized>(
    value: &T,
    config: &LpgConfig,
    description: &str,
) -> Result<String, ProjectionError> {
    String::from_utf8(canonical_json_bounded(value, config.limits(), description)?).map_err(
        |error| ProjectionError::integrity(format!("JSON encoder emitted non-UTF-8: {error}")),
    )
}

pub(super) fn parse_json<T: FromJson + ToJson>(
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
    let value: T = super::super::json_codec::from_slice(bytes).map_err(|error| {
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

/// Size of [`render_hex_blocks`]'s stack staging block, in rendered characters.
const HEX_BLOCK_BYTES: usize = 8_192;

/// Render `value` as lowercase hexadecimal — two characters per byte, zero
/// padded, leading zero bytes preserved — handing the rendered characters to
/// `emit` one fixed stack block at a time, so a byte sink receives the text
/// without an owned `String` in between.
///
/// Blocking rather than emitting per byte keeps the sink call count proportional
/// to the payload size divided by [`HEX_BLOCK_BYTES`], not to the byte count.
pub(super) fn render_hex_blocks(
    value: &[u8],
    mut emit: impl FnMut(&[u8]) -> Result<(), ProjectionError>,
) -> Result<(), ProjectionError> {
    let mut block = [0u8; HEX_BLOCK_BYTES];
    // Two output characters per source byte, so a block holds half its size in
    // source bytes and the rendering always fits.
    for source in value.chunks(HEX_BLOCK_BYTES / 2) {
        let rendered = purrdf_hash::hex::encode_to_slice(source, &mut block)
            .map_err(|error| ProjectionError::limit(error.to_string()))?;
        emit(rendered.as_bytes())?;
    }
    Ok(())
}

pub(super) struct BoundedText {
    bytes: Vec<u8>,
    limit: usize,
    description: &'static str,
    path: &'static str,
}

pub(super) trait LpgTextWriter {
    fn push(&mut self, value: &str) -> Result<(), ProjectionError>;
    fn push_hex(&mut self, value: &[u8]) -> Result<(), ProjectionError>;
}

impl<T: LpgTextWriter + ?Sized> LpgTextWriter for &mut T {
    fn push(&mut self, value: &str) -> Result<(), ProjectionError> {
        (**self).push(value)
    }

    fn push_hex(&mut self, value: &[u8]) -> Result<(), ProjectionError> {
        (**self).push_hex(value)
    }
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
        self.bytes.reserve(added);
        render_hex_blocks(value, |block| {
            self.bytes.extend_from_slice(block);
            Ok(())
        })
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

impl LpgTextWriter for BoundedText {
    fn push(&mut self, value: &str) -> Result<(), ProjectionError> {
        Self::push(self, value)
    }

    fn push_hex(&mut self, value: &[u8]) -> Result<(), ProjectionError> {
        Self::push_hex(self, value)
    }
}

pub(super) fn hex_decode(
    value: &str,
    description: &str,
    path: &str,
) -> Result<Vec<u8>, ProjectionError> {
    purrdf_hash::hex::decode_canonical(value).map_err(|error| {
        let reason = match error {
            purrdf_hash::hex::HexError::OddLength { .. } => {
                format!("{description} lowercase-hex payload has odd length")
            }
            _ => format!("{description} contains a non-lowercase-hex digit"),
        };
        ProjectionError::syntax(reason).at_path(path)
    })
}
