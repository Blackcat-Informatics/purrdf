// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Filesystem-free transactional sinks for bounded projection artifacts.

use super::{ProjectionError, ProjectionLimits, ProjectionPackage};

/// Transactional destination for path-delimited projection artifact chunks.
///
/// Writers call the methods in package order: begin package, then zero or more
/// begin/write/finish artifact groups, then commit. Any error calls
/// [`abort_package`](Self::abort_package). A sink must make committed packages
/// visible atomically and discard all package state on abort; it must not retain a
/// partial successful-looking carrier.
pub trait ProjectionArtifactSink {
    /// Begin one package transaction.
    ///
    /// # Errors
    ///
    /// Returns a typed sink failure before accepting artifacts.
    fn begin_package(&mut self) -> Result<(), ProjectionError>;

    /// Begin one safe POSIX-relative artifact path.
    ///
    /// # Errors
    ///
    /// Returns a typed sink failure before accepting body chunks.
    fn begin_artifact(&mut self, path: &str) -> Result<(), ProjectionError>;

    /// Append one non-empty artifact body chunk.
    ///
    /// # Errors
    ///
    /// Returns a typed sink failure. The writer immediately aborts the package.
    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), ProjectionError>;

    /// Finish the current artifact.
    ///
    /// # Errors
    ///
    /// Returns a typed sink failure. The writer immediately aborts the package.
    fn finish_artifact(&mut self) -> Result<(), ProjectionError>;

    /// Atomically commit the complete package.
    ///
    /// # Errors
    ///
    /// Returns a typed sink failure; the writer then aborts the transaction.
    fn commit_package(&mut self) -> Result<(), ProjectionError>;

    /// Infallibly discard the active package and any partial artifact.
    fn abort_package(&mut self);
}

/// In-memory adapter used by the materializing [`ProjectionPackage`] APIs.
#[derive(Debug)]
pub struct ProjectionPackageSink {
    limits: ProjectionLimits,
    package: ProjectionPackage,
    current: Option<(String, Vec<u8>)>,
    active: bool,
    committed: bool,
}

impl ProjectionPackageSink {
    /// Construct an idle in-memory sink under explicit package limits.
    pub fn new(limits: ProjectionLimits) -> Self {
        Self {
            limits,
            package: ProjectionPackage::new(limits),
            current: None,
            active: false,
            committed: false,
        }
    }

    /// Move out the atomically committed package.
    ///
    /// # Errors
    ///
    /// Returns a package-state error if no complete transaction was committed.
    pub fn into_package(self) -> Result<ProjectionPackage, ProjectionError> {
        if !self.committed || self.active || self.current.is_some() {
            return Err(ProjectionError::package(
                "projection package sink has no committed package",
            ));
        }
        Ok(self.package)
    }
}

impl ProjectionArtifactSink for ProjectionPackageSink {
    fn begin_package(&mut self) -> Result<(), ProjectionError> {
        if self.active || self.current.is_some() {
            return Err(ProjectionError::package(
                "projection package sink transaction is already active",
            ));
        }
        self.package = ProjectionPackage::new(self.limits);
        self.active = true;
        self.committed = false;
        Ok(())
    }

    fn begin_artifact(&mut self, path: &str) -> Result<(), ProjectionError> {
        if !self.active || self.current.is_some() {
            return Err(ProjectionError::package(
                "projection package sink cannot begin an artifact in its current state",
            ));
        }
        self.current = Some((path.to_owned(), Vec::new()));
        Ok(())
    }

    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), ProjectionError> {
        let Some((path, bytes)) = self.current.as_mut() else {
            return Err(ProjectionError::package(
                "projection package sink has no active artifact",
            ));
        };
        let length = bytes.len().checked_add(chunk.len()).ok_or_else(|| {
            ProjectionError::limit("artifact byte count overflow").at_path(path.as_str())
        })?;
        if length > self.limits.max_artifact_bytes() {
            return Err(ProjectionError::limit(format!(
                "artifact exceeds the {}-byte limit",
                self.limits.max_artifact_bytes()
            ))
            .at_path(path.as_str()));
        }
        bytes.extend_from_slice(chunk);
        Ok(())
    }

    fn finish_artifact(&mut self) -> Result<(), ProjectionError> {
        let Some((path, bytes)) = self.current.take() else {
            return Err(ProjectionError::package(
                "projection package sink has no active artifact",
            ));
        };
        self.package.insert(path, bytes)
    }

    fn commit_package(&mut self) -> Result<(), ProjectionError> {
        if !self.active || self.current.is_some() {
            return Err(ProjectionError::package(
                "projection package sink cannot commit an incomplete transaction",
            ));
        }
        self.active = false;
        self.committed = true;
        Ok(())
    }

    fn abort_package(&mut self) {
        self.package = ProjectionPackage::new(self.limits);
        self.current = None;
        self.active = false;
        self.committed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ProjectionLimits {
        ProjectionLimits::new(2, 8, 16, 2_560, 4).expect("limits")
    }

    #[test]
    fn package_sink_commits_atomically_and_abort_discards_partial_bytes() {
        let mut sink = ProjectionPackageSink::new(limits());
        sink.begin_package().expect("begin");
        sink.begin_artifact("a.txt").expect("artifact");
        sink.write_chunk(b"abc").expect("chunk");
        sink.finish_artifact().expect("finish artifact");
        sink.commit_package().expect("commit");
        let package = sink.into_package().expect("package");
        assert_eq!(package.get("a.txt"), Some(b"abc".as_slice()));

        let mut aborted = ProjectionPackageSink::new(limits());
        aborted.begin_package().expect("begin");
        aborted.begin_artifact("a.txt").expect("artifact");
        aborted.write_chunk(b"abc").expect("chunk");
        aborted.abort_package();
        assert!(aborted.into_package().is_err());
    }
}
