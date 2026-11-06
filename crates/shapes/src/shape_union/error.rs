// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native failures while discovering or reading the shape-file union.

use std::path::{Path, PathBuf};

use purrdf_hash::error::{ErrorValue, ErrorVisitor, IoFailure, StructuredError};

use crate::error::{FailureClass, ShapesError};

/// An original shape-file failure with its path and operation retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeFilesError {
    path: PathBuf,
    reason: Reason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Reason {
    MissingGenerated,
    Io(Operation, IoFailure),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Directory,
    Entry,
    SliceEntry,
    File,
}

impl ShapeFilesError {
    pub(super) fn missing_generated(path: PathBuf) -> ShapesError {
        ShapesError::ShapeFiles(Box::new(Self {
            path,
            reason: Reason::MissingGenerated,
        }))
    }

    fn io(path: &Path, operation: Operation, cause: std::io::Error) -> ShapesError {
        ShapesError::ShapeFiles(Box::new(Self {
            path: path.to_path_buf(),
            reason: Reason::Io(operation, cause.into()),
        }))
    }

    pub(super) fn directory(path: &Path, cause: std::io::Error) -> ShapesError {
        Self::io(path, Operation::Directory, cause)
    }

    pub(super) fn entry(path: &Path, cause: std::io::Error, slice: bool) -> ShapesError {
        Self::io(
            path,
            if slice {
                Operation::SliceEntry
            } else {
                Operation::Entry
            },
            cause,
        )
    }

    pub(super) fn read_file(path: &Path, cause: std::io::Error) -> ShapesError {
        Self::io(path, Operation::File, cause)
    }

    pub(crate) const fn failure_class(&self) -> FailureClass {
        match self.reason {
            Reason::MissingGenerated => FailureClass::Evaluation,
            Reason::Io(..) => FailureClass::Processor,
        }
    }
}

impl StructuredError for ShapeFilesError {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant("shacl-shape-files");
        visitor.value(
            Some("path"),
            ErrorValue::Bytes(self.path.as_os_str().as_encoded_bytes()),
        );
        let operation = match &self.reason {
            Reason::MissingGenerated => "missing-generated",
            Reason::Io(Operation::Directory, _) => "read-directory",
            Reason::Io(Operation::Entry, _) => "directory-entry",
            Reason::Io(Operation::SliceEntry, _) => "slice-directory-entry",
            Reason::Io(Operation::File, _) => "read-file",
        };
        visitor.value(Some("operation"), ErrorValue::Code(operation));
        visitor.value(
            Some("cause"),
            match &self.reason {
                Reason::MissingGenerated => ErrorValue::Absent,
                Reason::Io(_, cause) => ErrorValue::Cause(cause),
            },
        );
    }
}

impl std::fmt::Display for ShapeFilesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.reason {
            Reason::MissingGenerated => write!(
                f,
                "no generated shapes under {} — run `purrdf regenerate frame-shapes` (P11 enforcement lives there)",
                self.path.display()
            ),
            Reason::Io(Operation::Directory, cause) => {
                write!(f, "failed to read {}: {cause}", self.path.display())
            }
            Reason::Io(Operation::Entry, cause) => write!(
                f,
                "failed to read dir entry in {}: {cause}",
                self.path.display()
            ),
            Reason::Io(Operation::SliceEntry, cause) => write!(f, "dir entry error: {cause}"),
            Reason::Io(Operation::File, cause) => write!(
                f,
                "failed to read shape file {}: {cause}",
                self.path.display()
            ),
        }
    }
}

impl std::error::Error for ShapeFilesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.reason {
            Reason::MissingGenerated => None,
            Reason::Io(_, cause) => Some(cause),
        }
    }
}
