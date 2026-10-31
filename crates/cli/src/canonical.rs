// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit command-owned canonical-search policy and receipt publication.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use purrdf_core::CanonicalizationControl;

use crate::error::CliError;

pub(crate) struct Request {
    control: Option<CanonicalizationControl<'static>>,
    destination: Option<PathBuf>,
    demanded: bool,
    pending_reports: std::cell::RefCell<Vec<(crate::cli::ReportTarget, String)>>,
    published_runs: std::cell::Cell<Option<usize>>,
}

impl Request {
    pub(crate) fn new(policy: Option<&str>, destination: Option<&Path>) -> Result<Self, CliError> {
        match (policy, destination) {
            (None, None) => Ok(Self {
                control: None,
                destination: None,
                demanded: false,
                pending_reports: std::cell::RefCell::new(Vec::new()),
                published_runs: std::cell::Cell::new(None),
            }),
            (Some(policy), Some(destination)) => {
                let policy = purrdf_validate::canonical::parse_policy(policy)
                    .map_err(|error| CliError::Usage(format!("--canonical-search: {error}")))?;
                Ok(Self {
                    control: Some(CanonicalizationControl::new(policy, None)),
                    destination: Some(destination.to_owned()),
                    demanded: false,
                    pending_reports: std::cell::RefCell::new(Vec::new()),
                    published_runs: std::cell::Cell::new(None),
                })
            }
            _ => Err(CliError::Usage(
                "--canonical-search and --canonicalization-receipts DEST are required together"
                    .to_owned(),
            )),
        }
    }

    pub(crate) fn control(&mut self) -> Result<&mut CanonicalizationControl<'static>, CliError> {
        if self.published_runs.get().is_some() {
            return Err(CliError::Runtime("canonical request invariant: a canonical run was requested after receipt publication; prepare every canonical result before publishing output".to_owned()));
        }
        self.demanded = true;
        self.control.as_mut().ok_or_else(|| CliError::Usage(
            "this operation canonicalizes RDF; supply --canonical-search unlimited or a canonical decimal u64 and --canonicalization-receipts DEST ('-' writes stderr)".to_owned()))
    }

    /// All known command inputs and outputs are protected before any scan begins.
    pub(crate) fn protect(&self, paths: &[PathBuf]) -> Result<(), CliError> {
        let Some(destination) = self
            .destination
            .as_deref()
            .filter(|path| *path != Path::new("-"))
        else {
            return Ok(());
        };
        let resolved = resolve(destination)?;
        for path in paths.iter().filter(|path| path.as_path() != Path::new("-")) {
            if resolved == resolve(path)? || same_file(destination, path)? {
                return Err(CliError::Usage(format!(
                    "--canonicalization-receipts {} aliases input or output {}; choose a distinct destination",
                    destination.display(),
                    path.display()
                )));
            }
        }
        Ok(())
    }

    /// Refuse unused supplied controls before any raw output, then publish all finalized runs.
    pub(crate) fn before_output(&self) -> Result<(), CliError> {
        if self.destination.is_some() && !self.demanded {
            return Err(CliError::Usage("the selected path performs no canonicalization; remove --canonical-search and --canonicalization-receipts".to_owned()));
        }
        self.publish()?;
        self.flush_reports()
    }

    pub(crate) fn defer_report(
        &self,
        target: &crate::cli::ReportTarget,
        rendered: &str,
    ) -> Result<(), CliError> {
        if matches!(target, crate::cli::ReportTarget::Silent) {
            return Ok(());
        }
        if self.published_runs.get().is_some() {
            return Err(CliError::Runtime(
                "canonical request invariant: a report was queued after receipt publication"
                    .to_owned(),
            ));
        }
        self.pending_reports
            .borrow_mut()
            .push((target.clone(), rendered.to_owned()));
        Ok(())
    }

    fn flush_reports(&self) -> Result<(), CliError> {
        for (target, rendered) in self.pending_reports.borrow_mut().drain(..) {
            crate::report::publish_rendered(&target, &rendered)?;
        }
        Ok(())
    }

    fn publish(&self) -> Result<(), CliError> {
        let (Some(control), Some(destination)) = (&self.control, &self.destination) else {
            return Ok(());
        };
        if let Some(published) = self.published_runs.get() {
            if published != control.receipts().len() {
                return Err(CliError::Runtime(
                    "canonical request invariant: finalized receipts changed after publication"
                        .to_owned(),
                ));
            }
            return Ok(());
        }
        let receipts = purrdf_validate::canonical::receipts_json(control.receipts()).to_string();
        let write = || -> std::io::Result<()> {
            if destination == Path::new("-") {
                let mut stderr = std::io::stderr().lock();
                stderr.write_all(receipts.as_bytes())?;
                stderr.write_all(b"\n")?;
                stderr.flush()
            } else {
                let mut file = std::fs::File::create(destination)?;
                file.write_all(receipts.as_bytes())?;
                file.write_all(b"\n")?;
                file.flush()
            }
        };
        write().map_err(|cause| CliError::ReceiptPublication {
            destination: destination.clone(),
            cause,
            original: None,
        })?;
        self.published_runs.set(Some(control.receipts().len()));
        Ok(())
    }

    /// Refusal publication never replaces the original native refusal owner.
    pub(crate) fn finish<T>(&self, result: Result<T, CliError>) -> Result<T, CliError> {
        match result {
            Ok(value) => {
                self.before_output()?;
                Ok(value)
            }
            Err(error @ CliError::ReceiptPublication { .. }) => Err(error),
            Err(original) => match self.publish().and_then(|()| self.flush_reports()) {
                Ok(()) => Err(original),
                Err(CliError::ReceiptPublication {
                    destination, cause, ..
                }) => Err(CliError::ReceiptPublication {
                    destination,
                    cause,
                    original: Some(Box::new(original)),
                }),
                Err(failure) => Err(CliError::OutputPublication {
                    original: Box::new(original),
                    failure: Box::new(failure),
                }),
            },
        }
    }
}

/// Resolve existing symlinks and a new output's existing parent, without guessing on I/O refusal.
fn resolve(path: &Path) -> Result<PathBuf, CliError> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let name = path.file_name().ok_or_else(|| {
                CliError::Usage(format!(
                    "invalid receipt/input/output path {}",
                    path.display()
                ))
            })?;
            Ok(parent.canonicalize()?.join(name))
        }
        Err(error) => Err(error.into()),
    }
}

fn same_file(a: &Path, b: &Path) -> Result<bool, CliError> {
    let metadata = |path: &Path| -> Result<Option<std::fs::Metadata>, CliError> {
        match std::fs::metadata(path) {
            Ok(metadata) => Ok(Some(metadata)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    };
    let (Some(a), Some(b)) = (metadata(a)?, metadata(b)?) else {
        return Ok(false);
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = (a, b);
        Err(CliError::Usage("cannot prove distinct identities for an existing receipt file on this platform; choose a new receipt destination".to_owned()))
    }
}

/// Capture the native owner before retaining a command's established contextual diagnostic.
pub(crate) fn map_error<E: std::error::Error + 'static>(
    error: E,
    map: impl FnOnce(E) -> CliError,
) -> CliError {
    let cause = purrdf_validate::canonical::native_canonical_cause(&error).cloned();
    let mapped = map(error);
    match cause {
        Some(cause) => CliError::Canonical {
            message: mapped.to_string(),
            cause,
        },
        None => mapped,
    }
}
