// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ordered pattern sets over the single compiled language and execution engine.

use super::{
    Account, Admission, CompileError, IriComponent, MatchError, Mode, Range, Resident,
    StorageError, Token, compile, execute, iri_surface,
};

/// Original pattern failure or physical refusal to allocate the complete set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetCompileError {
    /// A source pattern failed validation or construction.
    Pattern {
        /// Zero-based source-pattern index, including duplicates.
        index: usize,
        /// Original compiler refusal.
        cause: CompileError,
    },
    /// Native storage refusal for the set's combined arrays.
    Storage(StorageError),
}
impl std::fmt::Display for SetCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern { index, cause } => write!(f, "glob pattern {index}: {cause}"),
            Self::Storage(cause) => cause.fmt(f),
        }
    }
}
impl std::error::Error for SetCompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pattern { cause, .. } => Some(cause),
            Self::Storage(cause) => Some(cause),
        }
    }
}
purrdf_lex::variant_from!(SetCompileError { Storage(StorageError) });
impl purrdf_hash::error::StructuredError for SetCompileError {
    purrdf_hash::error_visit! {
        Pattern { index, cause } => "glob-set-pattern-compile" {
            "index" => Unsigned(*index as u128), "cause" => Cause(cause)
        },
        Storage(cause) => "glob-set-storage" { "cause" => Cause(cause) },
    }
}

/// Refusal to return a complete ordered pattern-set result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetMatchError {
    /// Workspace must hold the worst case: one index for every source pattern.
    Workspace {
        /// Number of index slots required.
        required: usize,
        /// Number of index slots supplied.
        provided: usize,
    },
    /// The aggregate work allowance refused a unit in this pattern.
    Pattern {
        /// Zero-based source-pattern index.
        index: usize,
        /// Original matcher refusal, with aggregate consumed work and limit.
        cause: MatchError,
    },
}
impl std::fmt::Display for SetMatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workspace { required, provided } => write!(
                f,
                "glob set requires {required} index slots, {provided} supplied"
            ),
            Self::Pattern { index, cause } => write!(f, "glob pattern {index}: {cause}"),
        }
    }
}
impl std::error::Error for SetMatchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Workspace { .. } => None,
            Self::Pattern { cause, .. } => Some(cause),
        }
    }
}
impl purrdf_hash::error::StructuredError for SetMatchError {
    purrdf_hash::error_visit! {
        Workspace { required, provided } => "glob-set-workspace" {
            "required" => Unsigned(*required as u128), "provided" => Unsigned(*provided as u128)
        },
        Pattern { index, cause } => "glob-set-pattern-match" {
            "index" => Unsigned(*index as u128), "cause" => Cause(cause)
        },
    }
}

/// A complete ordered result borrowing the caller's workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetMatchReport<'a> {
    /// Every matching source index in order, including duplicate patterns.
    pub indices: &'a [usize],
    /// Aggregate deterministic work consumed by all patterns.
    pub work: u64,
}

/// Immutable ordered programs under one physical admission account.
#[derive(Debug)]
pub struct CompiledGlobSet<S: Admission = Resident> {
    // Payloads die before their account, including on unwind.
    pub(super) tokens: Vec<Token>,
    pub(super) ranges: Vec<Range>,
    pub(super) ends: Vec<usize>,
    pub(super) mode: Mode,
    pub(super) _account: Account<S>,
}
impl CompiledGlobSet {
    /// Compile an ordered set with resident physical storage.
    /// # Errors
    /// Preserves native syntax and storage refusals.
    pub fn compile(patterns: &[impl AsRef<str>], mode: Mode) -> Result<Self, SetCompileError> {
        Self::compile_with_storage(patterns, mode, Resident)
    }
}
impl<S: Admission> CompiledGlobSet<S> {
    /// Validate all patterns, then admit and construct one set of flat arrays.
    /// The dedicated, initially empty account lives until all payloads die.
    /// # Errors
    /// A pattern refusal retains its source index and original cause; aggregate
    /// storage refusal retains its native cause without fabricating an index.
    pub fn compile_with_storage(
        patterns: &[impl AsRef<str>],
        mode: Mode,
        storage: S,
    ) -> Result<Self, SetCompileError> {
        compile::compile_set(patterns, mode, storage)
    }

    /// Return all matching source indices without allocating during execution.
    /// Workspace needs one slot per source pattern, even if fewer match.
    /// # Errors
    /// Workspace refusal precedes execution. Work exhaustion returns no report;
    /// workspace contents on refusal are scratch, never a completed answer.
    pub fn match_literal<'a>(
        &self,
        lexical: &str,
        max_work: u64,
        workspace: &'a mut [usize],
    ) -> Result<SetMatchReport<'a>, SetMatchError> {
        self.matches(Some(lexical), max_work, workspace)
    }

    /// Match an exact IRI surface without normalization or percent decoding.
    /// Absent components cost one negative selection per source pattern.
    /// # Errors
    /// Returns original workspace or aggregate work refusal, with no report.
    pub fn match_iri<'a>(
        &self,
        iri: &purrdf_iri::Iri,
        component: IriComponent,
        max_work: u64,
        workspace: &'a mut [usize],
    ) -> Result<SetMatchReport<'a>, SetMatchError> {
        self.matches(iri_surface(iri, component), max_work, workspace)
    }

    fn matches<'a>(
        &self,
        lexical: Option<&str>,
        max_work: u64,
        workspace: &'a mut [usize],
    ) -> Result<SetMatchReport<'a>, SetMatchError> {
        if workspace.len() < self.ends.len() {
            return Err(SetMatchError::Workspace {
                required: self.ends.len(),
                provided: workspace.len(),
            });
        }
        let mut meter = execute::Meter::new(max_work);
        let mut start = 0;
        let mut matched = 0;
        for (index, &end) in self.ends.iter().enumerate() {
            let result = if let Some(text) = lexical {
                execute::matches_metered(
                    &self.tokens[start..end],
                    &self.ranges,
                    self.mode,
                    text,
                    &mut meter,
                )
                .map(|report| report.matched)
            } else {
                meter.tick(0).map(|()| false)
            }
            .map_err(|cause| SetMatchError::Pattern { index, cause })?;
            if result {
                workspace[matched] = index;
                matched += 1;
            }
            start = end;
        }
        Ok(SetMatchReport {
            indices: &workspace[..matched],
            work: meter.work,
        })
    }
}
