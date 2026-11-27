// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Root-relative file expansion over a caller-owned directory source.

use core::error::Error;
use core::fmt;
use core::ops::Range;
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_lex::allocation::Memory;

use super::{
    Account, Admission, CompiledGlobSet, MatchError, Mode, Resident, SetMatchError, StorageError,
    execute::Meter,
};

/// Kind of an observed directory entry. The source must not follow links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// An ordinary file, eligible to match.
    File,
    /// An ordinary directory, eligible to traverse.
    Directory,
    /// A symbolic or analogous indirection. Expansion refuses it.
    Link,
    /// An object other than a regular file, directory, or link.
    Other,
}

/// One observed direct child, borrowing the cursor's original name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry<'a> {
    /// One exact UTF-8 component, with no separator or special dot component.
    pub name: &'a str,
    /// The object's own kind, without resolving indirections.
    pub kind: EntryKind,
}

/// Pull children of one directory, without following links.
pub trait Directory {
    /// Original provider failure.
    type Error: Error + 'static;

    /// Return the next observed child. Only a completed directory returns `None`.
    /// # Errors
    /// Returns the original enumeration or classification failure.
    fn next_entry(&mut self) -> Result<Option<Entry<'_>>, Self::Error>;
}

/// Callable directory authority rooted by the caller. Traversal never reads
/// file contents. An adapter must enforce its own confinement and report any
/// missing or unavailable directory as an error, not as an empty cursor.
pub trait Source {
    /// Original provider failure.
    type Error: Error + 'static;
    /// Cursor borrowing this source for one directory at a time.
    type Cursor<'a>: Directory<Error = Self::Error>
    where
        Self: 'a;

    /// Open a validated relative directory. The empty string selects the root.
    /// # Errors
    /// Returns the original open or containment failure.
    fn open_directory(&mut self, relative: &str) -> Result<Self::Cursor<'_>, Self::Error>;
}

/// Why expansion refused to publish a complete result.
#[derive(Debug)]
#[non_exhaustive]
pub enum Failure<E> {
    /// The supplied set was compiled for text rather than paths.
    TextMode,
    /// A directory reported an invalid direct component.
    Component,
    /// A link or unsupported object was encountered.
    Unsupported(EntryKind),
    /// Repeated observations of a path disagree about its kind.
    ConflictingKind,
    /// The provider refused an open or a cursor read.
    Source(E),
    /// The shared traversal and matching budget refused a traversal unit.
    Work(MatchError),
    /// The same budget refused work in a source pattern.
    Match(SetMatchError),
    /// Physical admission, layout, or allocation refused.
    Storage(StorageError),
}

#[derive(Debug)]
struct Fault<E> {
    path: String,
    failure: Failure<E>,
}

/// Failure with the relative directory or entry at which it occurred.
pub struct ExpansionError<E, A: Admission = Resident> {
    fault: Fault<E>,
    _account: Account<A>,
}

impl<E, A: Admission> ExpansionError<E, A> {
    /// Empty denotes the root or a failure before entering it.
    pub fn path(&self) -> &str {
        &self.fault.path
    }

    /// Original cause or failing invariant.
    pub fn failure(&self) -> &Failure<E> {
        &self.fault.failure
    }
}

impl<E: fmt::Debug, A: Admission> fmt::Debug for ExpansionError<E, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionError")
            .field("path", &self.fault.path)
            .field("failure", &self.fault.failure)
            .finish_non_exhaustive()
    }
}

impl<E: fmt::Display, A: Admission> fmt::Display for ExpansionError<E, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "filesystem glob at {:?}: ", self.path())?;
        match self.failure() {
            Failure::TextMode => f.write_str("filesystem expansion requires path mode"),
            Failure::Component => f.write_str("source returned an invalid direct child component"),
            Failure::Unsupported(kind) => write!(f, "source returned an unsupported {kind:?}"),
            Failure::ConflictingKind => f.write_str("repeated entries disagree about object kind"),
            Failure::Source(cause) => cause.fmt(f),
            Failure::Work(cause) => cause.fmt(f),
            Failure::Match(cause) => cause.fmt(f),
            Failure::Storage(cause) => cause.fmt(f),
        }
    }
}

impl<E: Error + 'static, A: Admission> Error for ExpansionError<E, A> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.failure() {
            Failure::Source(cause) => Some(cause),
            Failure::Work(cause) => Some(cause),
            Failure::Match(cause) => Some(cause),
            Failure::Storage(cause) => Some(cause),
            _ => None,
        }
    }
}

impl<E: StructuredError, A: Admission> StructuredError for ExpansionError<E, A> {
    fn visit<'e>(&'e self, visitor: &mut dyn ErrorVisitor<'e>) {
        visitor.variant("glob-filesystem-expansion");
        visitor.value(Some("path"), ErrorValue::Text(self.path()));
        let reason = match self.failure() {
            Failure::TextMode => "text-mode",
            Failure::Component => "invalid-component",
            Failure::Unsupported(kind) => {
                let kind = match kind {
                    EntryKind::File => "file",
                    EntryKind::Directory => "directory",
                    EntryKind::Link => "link",
                    EntryKind::Other => "other",
                };
                visitor.value(Some("kind"), ErrorValue::Code(kind));
                "unsupported-kind"
            }
            Failure::ConflictingKind => "conflicting-kind",
            Failure::Source(error) => {
                visitor.value(Some("cause"), ErrorValue::Cause(error));
                "source"
            }
            Failure::Work(error) => {
                visitor.value(Some("cause"), ErrorValue::Cause(error));
                "traversal-work"
            }
            Failure::Match(error) => {
                visitor.value(Some("cause"), ErrorValue::Cause(error));
                "matching-work"
            }
            Failure::Storage(error) => {
                visitor.value(Some("cause"), ErrorValue::Cause(error));
                "storage"
            }
        };
        visitor.value(Some("reason"), ErrorValue::Code(reason));
    }
}

/// One file and the original pattern occurrences that matched it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMatch<'a> {
    /// Root-relative path with exact UTF-8 components separated by slash.
    pub path: &'a str,
    /// Matching source indices in pattern order, including repeated patterns.
    pub indices: &'a [usize],
}

/// A complete expansion. Its payload is released before its admission owner.
#[derive(Debug)]
pub struct Expansion<A: Admission = Resident> {
    files: Vec<(String, Range<usize>)>,
    indices: Vec<usize>,
    work: u64,
    _account: Account<A>,
}

impl<A: Admission> Expansion<A> {
    /// Files in depth-first order, sorting siblings by original UTF-8 bytes.
    pub fn files(&self) -> impl ExactSizeIterator<Item = FileMatch<'_>> {
        self.files.iter().map(|(path, range)| FileMatch {
            path,
            indices: &self.indices[range.clone()],
        })
    }

    /// Total charged work, including directory opens, cursor calls and matches.
    pub fn work(&self) -> u64 {
        self.work
    }
}

struct Pending {
    path: String,
    kind: EntryKind,
}

fn stable_component(name: &str) -> bool {
    !matches!(name, "" | "." | "..") && !name.contains(['/', '\0'])
}

fn read_directory<S: Source>(
    source: &mut S,
    directory: &str,
    meter: &mut Meter,
    memory: &mut Memory<'_, impl Admission>,
    siblings: &mut Vec<Pending>,
) -> Result<(), Failure<S::Error>> {
    meter.tick(0).map_err(Failure::Work)?;
    let mut cursor = source.open_directory(directory).map_err(Failure::Source)?;
    loop {
        meter.tick(0).map_err(Failure::Work)?;
        let Some(entry) = cursor.next_entry().map_err(Failure::Source)? else {
            return Ok(());
        };
        if !stable_component(entry.name) {
            return Err(Failure::Component);
        }
        let mut path = String::new();
        let separator = usize::from(!directory.is_empty());
        let length = directory
            .len()
            .checked_add(separator)
            .and_then(|start| start.checked_add(entry.name.len()))
            .ok_or(Failure::Storage(StorageError::SizeOverflow))?;
        memory
            .reserve_string(&mut path, length)
            .map_err(Failure::Storage)?;
        path.push_str(directory);
        if separator != 0 {
            path.push('/');
        }
        path.push_str(entry.name);
        memory
            .push(
                siblings,
                Pending {
                    path,
                    kind: entry.kind,
                },
            )
            .map_err(Failure::Storage)?;
    }
}

fn push_children<E>(
    siblings: &mut Vec<Pending>,
    pending: &mut Vec<Pending>,
    memory: &mut Memory<'_, impl Admission>,
) -> Result<(), Fault<E>> {
    siblings.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    let before = pending.len();
    while let Some(entry) = siblings.pop() {
        if pending.len() > before {
            let previous = &pending[pending.len() - 1];
            if previous.path == entry.path {
                if previous.kind != entry.kind {
                    return Err(Fault {
                        path: entry.path,
                        failure: Failure::ConflictingKind,
                    });
                }
                memory.release_string(entry.path).map_err(storage_error)?;
                continue;
            }
        }
        memory.push(pending, entry).map_err(storage_error)?;
    }
    Ok(())
}

fn storage_error<E>(cause: StorageError) -> Fault<E> {
    Fault {
        path: String::new(),
        failure: Failure::Storage(cause),
    }
}

#[derive(Debug, Default)]
struct Matches {
    files: Vec<(String, Range<usize>)>,
    indices: Vec<usize>,
}

impl<S: Admission> CompiledGlobSet<S> {
    /// Traverse a rooted source once with one budget for all opens, cursor reads
    /// and pattern work. Charge before every open or read, including an
    /// end-of-directory read. The memory account covers core scratch and results.
    /// # Errors
    /// Refuses text mode, invalid components, links, conflicting observations,
    /// source faults and work or storage failures without publishing a partial result.
    pub fn expand_files<T: Source, A: Admission>(
        &self,
        source: &mut T,
        max_work: u64,
        storage: A,
    ) -> Result<Expansion<A>, ExpansionError<T::Error, A>> {
        let mut account = Account(storage);
        let mut meter = Meter::new(max_work);
        match self.expand_files_in(source, &mut meter, &mut Memory::new(&mut account.0)) {
            Ok(Matches { files, indices }) => Ok(Expansion {
                files,
                indices,
                work: meter.work,
                _account: account,
            }),
            Err(fault) => {
                // All scratch has died. A sticky refund refusal cannot replace
                // the original cause or detach ownership of the error's path.
                let _ = account.0.resize(fault.path.capacity());
                Err(ExpansionError {
                    fault,
                    _account: account,
                })
            }
        }
    }

    fn expand_files_in<T: Source>(
        &self,
        source: &mut T,
        meter: &mut Meter,
        memory: &mut Memory<'_, impl Admission>,
    ) -> Result<Matches, Fault<T::Error>> {
        if self.mode != Mode::Path {
            return Err(Fault {
                path: String::new(),
                failure: Failure::TextMode,
            });
        }
        let mut files = Vec::new();
        let mut indices = Vec::new();
        let mut slots = Vec::new();
        let mut pending = Vec::new();
        let mut siblings = Vec::new();
        memory
            .reserve(&mut slots, self.programs.len())
            .map_err(storage_error)?;
        slots.resize(self.programs.len(), 0);
        let mut next = Some(Pending {
            path: String::new(),
            kind: EntryKind::Directory,
        });
        while let Some(Pending { path, kind }) = next {
            match kind {
                EntryKind::Directory => {
                    if let Err(failure) =
                        read_directory(source, &path, meter, memory, &mut siblings)
                    {
                        return Err(Fault { path, failure });
                    }
                    memory.release_string(path).map_err(storage_error)?;
                    push_children(&mut siblings, &mut pending, memory)?;
                }
                EntryKind::File => {
                    let report = match self.matches_metered(Some(&path), meter, &mut slots) {
                        Ok(report) => report,
                        Err(cause) => {
                            return Err(Fault {
                                path,
                                failure: Failure::Match(cause),
                            });
                        }
                    };
                    if report.indices.is_empty() {
                        memory.release_string(path).map_err(storage_error)?;
                    } else {
                        let start = indices.len();
                        memory
                            .extend(&mut indices, report.indices.iter().copied())
                            .map_err(storage_error)?;
                        memory
                            .push(&mut files, (path, start..indices.len()))
                            .map_err(storage_error)?;
                    }
                }
                kind => {
                    return Err(Fault {
                        path,
                        failure: Failure::Unsupported(kind),
                    });
                }
            }
            next = pending.pop();
        }
        memory.release_vec(slots).map_err(storage_error)?;
        memory.release_vec(pending).map_err(storage_error)?;
        memory.release_vec(siblings).map_err(storage_error)?;
        Ok(Matches { files, indices })
    }
}
