// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use super::ProjectionError;

/// Maximum artifact path length accepted by the portable package profile.
const MAX_ARTIFACT_PATH_BYTES: usize = 4_096;
/// Hard recursion ceiling that keeps all recursive term operations stack-bounded.
const MAX_TERM_DEPTH: usize = 128;
/// Largest value representable by the canonical 11-octal-digit USTAR size field.
const USTAR_MAX_MEMBER_BYTES: u64 = 0o77_777_777_777;

/// Mandatory resource bounds shared by projection writers and readers.
///
/// There is deliberately no `Default`: the caller chooses explicit limits suitable
/// for its trust and memory boundary. Deserialization validates the same invariants as
/// [`ProjectionLimits::new`], so a configuration file cannot bypass them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProjectionLimits {
    #[serde(rename = "max_artifacts")]
    artifact_count: usize,
    #[serde(rename = "max_artifact_bytes")]
    artifact_bytes: usize,
    #[serde(rename = "max_total_bytes")]
    total_bytes: usize,
    #[serde(rename = "max_archive_bytes")]
    archive_bytes: usize,
    #[serde(rename = "max_term_depth")]
    term_depth: usize,
}

impl ProjectionLimits {
    /// Construct validated projection limits.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionErrorKind::Configuration`](super::ProjectionErrorKind::Configuration)
    /// when a bound is zero, internally contradictory, or exceeds USTAR's member-size
    /// representation.
    pub fn new(
        max_artifacts: usize,
        max_artifact_bytes: usize,
        max_total_bytes: usize,
        max_archive_bytes: usize,
        max_term_depth: usize,
    ) -> Result<Self, ProjectionError> {
        if [
            max_artifacts,
            max_artifact_bytes,
            max_total_bytes,
            max_archive_bytes,
            max_term_depth,
        ]
        .contains(&0)
        {
            return Err(ProjectionError::configuration(
                "every projection resource limit must be greater than zero",
            ));
        }
        if max_artifact_bytes > max_total_bytes {
            return Err(ProjectionError::configuration(
                "max_artifact_bytes must not exceed max_total_bytes",
            ));
        }
        if max_total_bytes > max_archive_bytes {
            return Err(ProjectionError::configuration(
                "max_total_bytes must not exceed max_archive_bytes",
            ));
        }
        if max_archive_bytes < 1_536 {
            return Err(ProjectionError::configuration(
                "max_archive_bytes must allow one USTAR header and the two-block trailer (1536 bytes)",
            ));
        }
        if max_artifact_bytes as u64 > USTAR_MAX_MEMBER_BYTES {
            return Err(ProjectionError::configuration(format!(
                "max_artifact_bytes exceeds USTAR's {USTAR_MAX_MEMBER_BYTES}-byte member ceiling"
            )));
        }
        if max_term_depth > MAX_TERM_DEPTH {
            return Err(ProjectionError::configuration(format!(
                "max_term_depth exceeds the hard safety ceiling of {MAX_TERM_DEPTH}"
            )));
        }
        Ok(Self {
            artifact_count: max_artifacts,
            artifact_bytes: max_artifact_bytes,
            total_bytes: max_total_bytes,
            archive_bytes: max_archive_bytes,
            term_depth: max_term_depth,
        })
    }

    /// Maximum number of package artifacts.
    pub const fn max_artifacts(self) -> usize {
        self.artifact_count
    }

    /// Maximum bytes in one artifact.
    pub const fn max_artifact_bytes(self) -> usize {
        self.artifact_bytes
    }

    /// Maximum sum of artifact body bytes.
    pub const fn max_total_bytes(self) -> usize {
        self.total_bytes
    }

    /// Maximum encoded archive bytes accepted by a reader.
    pub const fn max_archive_bytes(self) -> usize {
        self.archive_bytes
    }

    /// Maximum recursive RDF triple-term depth.
    pub const fn max_term_depth(self) -> usize {
        self.term_depth
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectionLimits {
    #[serde(rename = "max_artifacts")]
    artifact_count: usize,
    #[serde(rename = "max_artifact_bytes")]
    artifact_bytes: usize,
    #[serde(rename = "max_total_bytes")]
    total_bytes: usize,
    #[serde(rename = "max_archive_bytes")]
    archive_bytes: usize,
    #[serde(rename = "max_term_depth")]
    term_depth: usize,
}

impl<'de> Deserialize<'de> for ProjectionLimits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawProjectionLimits::deserialize(deserializer)?;
        Self::new(
            raw.artifact_count,
            raw.artifact_bytes,
            raw.total_bytes,
            raw.archive_bytes,
            raw.term_depth,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// A deterministic, validated, filesystem-free projection artifact package.
///
/// Paths are safe POSIX-relative names and iterate lexically. Duplicate paths and
/// resource-limit breaches are hard errors. [`to_ustar`](Self::to_ustar) is
/// byte-deterministic; [`from_ustar`](Self::from_ustar) accepts only that canonical
/// encoding, which rejects checksum/header/order/trailer drift rather than silently
/// normalizing attacker-controlled archives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionPackage {
    artifacts: BTreeMap<String, Vec<u8>>,
    total_bytes: usize,
    archive_bytes: usize,
    limits: ProjectionLimits,
}

impl ProjectionPackage {
    /// Construct an empty package under explicit resource limits.
    pub fn new(limits: ProjectionLimits) -> Self {
        Self {
            artifacts: BTreeMap::new(),
            total_bytes: 0,
            archive_bytes: 1_024,
            limits,
        }
    }

    /// Construct a package from artifact pairs.
    ///
    /// # Errors
    ///
    /// Returns a typed package or resource-limit error on the first invalid pair.
    pub fn from_artifacts<I, P, B>(
        limits: ProjectionLimits,
        artifacts: I,
    ) -> Result<Self, ProjectionError>
    where
        I: IntoIterator<Item = (P, B)>,
        P: Into<String>,
        B: Into<Vec<u8>>,
    {
        let mut package = Self::new(limits);
        for (path, bytes) in artifacts {
            package.insert(path, bytes)?;
        }
        Ok(package)
    }

    /// Insert one artifact while preserving all path, uniqueness, and size invariants.
    ///
    /// # Errors
    ///
    /// Returns a typed package or resource-limit error without changing the package.
    pub fn insert(
        &mut self,
        path: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<(), ProjectionError> {
        let path = path.into();
        let bytes = bytes.into();
        let (total, archive_bytes) = self.validate_candidate(&path, bytes.len())?;
        self.commit(path, bytes, total, archive_bytes);
        Ok(())
    }

    fn insert_borrowed(&mut self, path: &str, bytes: &[u8]) -> Result<(), ProjectionError> {
        let (total, archive_bytes) = self.validate_candidate(path, bytes.len())?;
        self.commit(path.to_owned(), bytes.to_vec(), total, archive_bytes);
        Ok(())
    }

    fn validate_candidate(
        &self,
        path: &str,
        body_len: usize,
    ) -> Result<(usize, usize), ProjectionError> {
        validate_artifact_path(path)?;
        if self.artifacts.contains_key(path) {
            return Err(ProjectionError::package("duplicate artifact path").at_path(path));
        }
        if self.artifacts.len() >= self.limits.artifact_count {
            return Err(ProjectionError::limit(format!(
                "package exceeds its {}-artifact limit",
                self.limits.artifact_count
            )));
        }
        if body_len > self.limits.artifact_bytes {
            return Err(ProjectionError::limit(format!(
                "artifact is {} bytes; per-artifact limit is {}",
                body_len, self.limits.artifact_bytes
            ))
            .at_path(path));
        }
        let total = self
            .total_bytes
            .checked_add(body_len)
            .ok_or_else(|| ProjectionError::limit("package byte count overflow"))?;
        if total > self.limits.total_bytes {
            return Err(ProjectionError::limit(format!(
                "package bodies total {total} bytes; limit is {}",
                self.limits.total_bytes
            ))
            .at_path(path));
        }
        let archive_bytes = self
            .archive_bytes
            .checked_add(encoded_member_len(path.len(), body_len)?)
            .ok_or_else(|| ProjectionError::limit("archive byte count overflow"))?;
        if archive_bytes > self.limits.archive_bytes {
            return Err(ProjectionError::limit(format!(
                "canonical archive would be {archive_bytes} bytes; limit is {}",
                self.limits.archive_bytes
            ))
            .at_path(path));
        }
        Ok((total, archive_bytes))
    }

    fn commit(&mut self, path: String, bytes: Vec<u8>, total: usize, archive_bytes: usize) {
        self.total_bytes = total;
        self.archive_bytes = archive_bytes;
        self.artifacts.insert(path, bytes);
    }

    /// Borrow one artifact body by path.
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.artifacts.get(path).map(Vec::as_slice)
    }

    /// Artifacts in deterministic lexical path order.
    pub fn artifacts(&self) -> impl ExactSizeIterator<Item = (&str, &[u8])> {
        self.artifacts
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
    }

    /// Number of artifacts.
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    /// Whether the package has no artifacts.
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    /// Sum of artifact body bytes, excluding archive headers/padding.
    pub const fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Exact encoded archive size, including headers, padding, and trailer.
    pub const fn archive_bytes(&self) -> usize {
        self.archive_bytes
    }

    /// The resource limits governing this package.
    pub const fn limits(&self) -> ProjectionLimits {
        self.limits
    }

    /// Encode this package as canonical deterministic USTAR bytes.
    ///
    /// # Errors
    ///
    /// Returns a package error for an empty package or an archive construction error,
    /// and a resource-limit error when encoded bytes exceed `max_archive_bytes`.
    pub fn to_ustar(&self) -> Result<Vec<u8>, ProjectionError> {
        if self.is_empty() {
            return Err(ProjectionError::package(
                "a projection archive must contain at least one artifact",
            ));
        }
        let bytes = crate::ustar::write_archive_borrowed(self.artifacts())
            .map_err(ProjectionError::package)?;
        debug_assert_eq!(bytes.len(), self.archive_bytes);
        Ok(bytes)
    }

    /// Decode a canonical deterministic USTAR package under explicit limits.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-canonical, duplicate-path, unsafe-path, empty, or
    /// resource-exceeding archives. The canonical byte comparison validates all fixed
    /// USTAR header fields and checksums as well as lexical member order and trailer.
    pub fn from_ustar(bytes: &[u8], limits: ProjectionLimits) -> Result<Self, ProjectionError> {
        if bytes.len() > limits.max_archive_bytes() {
            return Err(ProjectionError::limit(format!(
                "archive is {} bytes; limit is {}",
                bytes.len(),
                limits.max_archive_bytes()
            )));
        }
        let mut package = Self::new(limits);
        for member in crate::ustar::archive_members(bytes) {
            let member = member.map_err(ProjectionError::package)?;
            package.insert_borrowed(member.name, member.data)?;
        }
        if package.is_empty() {
            return Err(ProjectionError::package(
                "a projection archive must contain at least one artifact",
            ));
        }
        let canonical = package.to_ustar()?;
        if canonical != bytes {
            return Err(ProjectionError::package(
                "archive is not in canonical PurRDF USTAR form",
            ));
        }
        Ok(package)
    }
}

fn encoded_member_len(path_bytes: usize, body_bytes: usize) -> Result<usize, ProjectionError> {
    let padded = |length: usize| {
        length
            .checked_add(511)
            .map(|rounded| (rounded / 512) * 512)
            .ok_or_else(|| ProjectionError::limit("USTAR padding length overflow"))
    };
    let regular = 512usize
        .checked_add(padded(body_bytes)?)
        .ok_or_else(|| ProjectionError::limit("USTAR member length overflow"))?;
    if path_bytes <= 100 {
        return Ok(regular);
    }
    let long_name_body = path_bytes
        .checked_add(1)
        .ok_or_else(|| ProjectionError::limit("USTAR long-name length overflow"))?;
    512usize
        .checked_add(padded(long_name_body)?)
        .and_then(|length| length.checked_add(regular))
        .ok_or_else(|| ProjectionError::limit("USTAR member length overflow"))
}

fn validate_artifact_path(path: &str) -> Result<(), ProjectionError> {
    if path.is_empty() {
        return Err(ProjectionError::package("artifact path is empty"));
    }
    if path.len() > MAX_ARTIFACT_PATH_BYTES {
        return Err(ProjectionError::package(format!(
            "artifact path is {} bytes; limit is {MAX_ARTIFACT_PATH_BYTES}",
            path.len()
        ))
        .at_path(path));
    }
    if path.starts_with('/') || path.contains('\\') || path.contains('\0') {
        return Err(ProjectionError::package(
            "artifact path must be a NUL-free POSIX-relative path",
        )
        .at_path(path));
    }
    if path.chars().any(char::is_control) {
        return Err(
            ProjectionError::package("artifact path must not contain control characters")
                .at_path(path),
        );
    }
    if path
        .split('/')
        .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(ProjectionError::package(
            "artifact path contains an empty, current-directory, or parent component",
        )
        .at_path(path));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ProjectionLimits {
        ProjectionLimits::new(8, 8_192, 32_768, 65_536, 16).expect("limits")
    }

    #[test]
    fn insertion_order_does_not_change_archive_bytes() {
        let a = ProjectionPackage::from_artifacts(
            limits(),
            [
                ("z/data.csv", b"z".to_vec()),
                ("a/meta.json", b"a".to_vec()),
            ],
        )
        .expect("package a");
        let b = ProjectionPackage::from_artifacts(
            limits(),
            [
                ("a/meta.json", b"a".to_vec()),
                ("z/data.csv", b"z".to_vec()),
            ],
        )
        .expect("package b");
        assert_eq!(
            a.to_ustar().expect("archive"),
            b.to_ustar().expect("archive")
        );
    }

    #[test]
    fn canonical_archive_round_trips() {
        let package = ProjectionPackage::from_artifacts(
            limits(),
            [
                ("graph/nodes.csv", b"id\n1\n".to_vec()),
                ("graph/edges.csv", b"source,target\n1,2\n".to_vec()),
            ],
        )
        .expect("package");
        let archive = package.to_ustar().expect("archive");
        assert_eq!(package.archive_bytes(), archive.len());
        let decoded = ProjectionPackage::from_ustar(&archive, limits()).expect("decode");
        assert_eq!(decoded, package);
        assert_eq!(decoded.to_ustar().expect("rewrite"), archive);
    }

    #[test]
    fn noncanonical_member_order_is_rejected() {
        let raw = crate::ustar::write_archive(&[
            ("z.csv".to_string(), b"z".to_vec()),
            ("a.csv".to_string(), b"a".to_vec()),
        ])
        .expect("archive");
        let error = ProjectionPackage::from_ustar(&raw, limits()).expect_err("must reject");
        assert_eq!(error.kind(), super::super::ProjectionErrorKind::Package);
        assert!(error.message().contains("not in canonical"));
    }

    #[test]
    fn header_mutation_is_rejected_by_canonical_comparison() {
        let package = ProjectionPackage::from_artifacts(limits(), [("a.csv", b"a".to_vec())])
            .expect("package");
        let mut raw = package.to_ustar().expect("archive");
        raw[136] = b'1';
        let error = ProjectionPackage::from_ustar(&raw, limits()).expect_err("must reject");
        assert!(error.message().contains("not in canonical"));
    }

    #[test]
    fn duplicate_and_unsafe_paths_are_rejected_without_mutation() {
        let mut package = ProjectionPackage::new(limits());
        package.insert("a.csv", b"a".to_vec()).expect("insert");
        assert!(package.insert("a.csv", b"b".to_vec()).is_err());
        assert!(package.insert("../escape.csv", b"b".to_vec()).is_err());
        assert_eq!(package.len(), 1);
        assert_eq!(package.get("a.csv"), Some(b"a".as_slice()));
    }

    #[test]
    fn limits_deserialize_through_validation() {
        let good = serde_json::to_string(&limits()).expect("serialize");
        assert_eq!(
            serde_json::from_str::<ProjectionLimits>(&good).expect("deserialize"),
            limits()
        );
        let bad = r#"{"max_artifacts":0,"max_artifact_bytes":1,"max_total_bytes":1,"max_archive_bytes":1,"max_term_depth":1}"#;
        assert!(serde_json::from_str::<ProjectionLimits>(bad).is_err());
        let unknown = r#"{"max_artifacts":1,"max_artifact_bytes":1,"max_total_bytes":1,"max_archive_bytes":1536,"max_term_depth":1,"surprise":true}"#;
        assert!(serde_json::from_str::<ProjectionLimits>(unknown).is_err());
        assert!(ProjectionLimits::new(1, 1, 1, 1_536, MAX_TERM_DEPTH + 1).is_err());
    }

    #[test]
    fn archive_overhead_limit_is_enforced_before_mutation() {
        let exact = ProjectionLimits::new(2, 1, 1, 1_536, 8).expect("limits");
        let mut package = ProjectionPackage::new(exact);
        package.insert("a", Vec::new()).expect("exact fit");
        assert_eq!(package.archive_bytes(), 1_536);

        let error = package
            .insert("b", Vec::new())
            .expect_err("second header exceeds archive limit");
        assert_eq!(
            error.kind(),
            super::super::ProjectionErrorKind::ResourceLimit
        );
        assert_eq!(package.len(), 1);
        assert_eq!(package.archive_bytes(), 1_536);
    }

    #[test]
    fn long_path_archive_size_includes_longlink_record() {
        let long_path = format!("{}.csv", "a".repeat(101));
        let package = ProjectionPackage::from_artifacts(
            ProjectionLimits::new(1, 1, 1, 4_096, 8).expect("limits"),
            [(long_path, vec![0])],
        )
        .expect("package");
        let archive = package.to_ustar().expect("archive");
        assert_eq!(package.archive_bytes(), 3_072);
        assert_eq!(package.archive_bytes(), archive.len());
    }
}
