// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Temporary directories and files under the cargo target directory.
//!
//! Scratch space lives under `target/`, never the system temporary directory:
//! it is then cleaned by `cargo clean`, it sits on the same file system as the
//! build, and nothing outside the workspace can remove it while a test holds
//! it. A name is `<prefix><pid>-<counter>-<nanos>`: the process id separates
//! concurrent test binaries, the process-wide monotonic counter separates
//! threads of one binary, and the wall-clock nanoseconds separate a binary
//! from a stale directory an earlier process with a recycled id left behind.
//! Creation is exclusive (`create_dir`, and `create_new` for files), so a name
//! that already exists is reported as an error — never adopted, never retried
//! under another name.
//!
//! Both types remove what they created when dropped. A removal that fails is a
//! panic, unless the thread is already unwinding, where a second panic would
//! abort the process and hide the first; the failure is then written to
//! standard error. A test that removed the path itself is not a failure.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The name prefix when the caller gives none.
const DEFAULT_PREFIX: &str = "purrdf-";

/// Separates concurrently created names within one process.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A directory removed, with everything under it, when this value is dropped.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// A fresh directory directly under `root`, which is created if absent.
    pub fn new_in(root: impl AsRef<Path>) -> io::Result<Self> {
        Self::with_prefix_in(DEFAULT_PREFIX, root)
    }

    /// A fresh directory under `root` whose name starts with `prefix`.
    ///
    /// A prefix holding a path separator, or one that is `.` or `..`, is
    /// refused: the directory would not be a direct child of `root`.
    pub fn with_prefix_in(prefix: &str, root: impl AsRef<Path>) -> io::Result<Self> {
        Self::create_at(fresh_path(prefix, root.as_ref())?)
    }

    /// Create exactly `path`; an existing entry there is a collision error.
    fn create_at(path: PathBuf) -> io::Result<Self> {
        std::fs::create_dir(&path).map_err(|error| collision_or(error, &path))?;
        Ok(Self { path })
    }

    /// A fresh directory for a crate's `src/` unit tests, under the `tmp`
    /// directory of the build directory the running test binary was built
    /// into.
    ///
    /// Cargo sets `CARGO_TARGET_TMPDIR` only for integration tests and benches,
    /// so a unit test derives the same place from its own executable. Cargo
    /// writes a `CACHEDIR.TAG` file at the root of every directory it builds
    /// into — the target directory, or a separate `build.build-dir` when one is
    /// configured — and the test binary lives somewhere beneath that root
    /// (`<root>/<profile>/deps/` in the classic layout, deeper in the
    /// per-package one). The nearest ancestor holding the tag is the root, and
    /// its `tmp` directory is the one `CARGO_TARGET_TMPDIR` names for a host
    /// build. An executable with no such ancestor is an error, never a
    /// fallback.
    pub fn for_unit_test() -> io::Result<Self> {
        Self::new_in(unit_test_root()?)
    }

    /// The directory's path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        settle_removal(&self.path, std::fs::remove_dir_all(&self.path));
    }
}

/// A file removed when this value is dropped. Writes go to the open handle.
#[derive(Debug)]
pub struct NamedTempFile {
    path: PathBuf,
    /// `Some` until `drop`, which closes the handle before removing the file
    /// (a platform may refuse to remove a file that is still open).
    file: Option<File>,
}

impl NamedTempFile {
    /// A fresh, empty file directly under `root`, which is created if absent,
    /// opened for reading and writing.
    pub fn new_in(root: impl AsRef<Path>) -> io::Result<Self> {
        Self::with_prefix_in(DEFAULT_PREFIX, root)
    }

    /// A fresh, empty file under `root` whose name starts with `prefix`, with
    /// the prefix rules of [`TempDir::with_prefix_in`].
    pub fn with_prefix_in(prefix: &str, root: impl AsRef<Path>) -> io::Result<Self> {
        Self::create_at(fresh_path(prefix, root.as_ref())?)
    }

    /// Create exactly `path`; an existing entry there is a collision error.
    fn create_at(path: PathBuf) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| collision_or(error, &path))?;
        Ok(Self {
            path,
            file: Some(file),
        })
    }

    /// A fresh file for a crate's `src/` unit tests, in the directory
    /// [`TempDir::for_unit_test`] uses.
    pub fn for_unit_test() -> io::Result<Self> {
        Self::new_in(unit_test_root()?)
    }

    /// The file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn handle(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("the handle is present until the value is dropped")
    }
}

impl Write for NamedTempFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.handle().write(buf)
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.handle().write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.handle().flush()
    }
}

impl Drop for NamedTempFile {
    fn drop(&mut self) {
        drop(self.file.take());
        settle_removal(&self.path, std::fs::remove_file(&self.path));
    }
}

/// `root/<prefix><pid>-<counter>-<nanos>`, after creating `root`.
fn fresh_path(prefix: &str, root: &Path) -> io::Result<PathBuf> {
    if prefix == "." || prefix == ".." || prefix.contains(['/', '\\']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("temporary name prefix {prefix:?} is not a plain file name"),
        ));
    }
    std::fs::create_dir_all(root)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| io::Error::other(format!("system clock is before the epoch: {error}")))?
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    Ok(root.join(format!("{prefix}{pid}-{counter}-{nanos}")))
}

/// A creation error, with an `AlreadyExists` one named as a collision.
fn collision_or(error: io::Error, path: &Path) -> io::Error {
    if error.kind() == io::ErrorKind::AlreadyExists {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "temporary path {} already exists: a name collision is an error, never adopted",
                path.display()
            ),
        )
    } else {
        error
    }
}

/// The file cargo writes at the root of every directory it builds into.
const CACHE_TAG: &str = "CACHEDIR.TAG";

/// The `tmp` directory of the build directory the running binary lives in.
fn unit_test_root() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    exe.ancestors()
        .skip(1)
        .find(|dir| dir.join(CACHE_TAG).is_file())
        .map(|root| root.join("tmp"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "{} is not inside a cargo build directory: no ancestor holds {CACHE_TAG}",
                    exe.display()
                ),
            )
        })
}

/// Report a failed removal: a panic, or a line on standard error while the
/// thread is already unwinding. A path that is already gone is not a failure.
fn settle_removal(path: &Path, result: io::Result<()>) {
    match result {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) if std::thread::panicking() => {
            eprintln!(
                "could not remove temporary path {}: {error}",
                path.display()
            );
        }
        Err(error) => panic!(
            "could not remove temporary path {}: {error}",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test_directories_live_in_the_target_tmp_directory() {
        let dir = TempDir::for_unit_test().expect("unit-test temp dir");
        let exe = std::env::current_exe().expect("current exe");
        let build_root = exe
            .ancestors()
            .find(|dir| dir.join(CACHE_TAG).is_file())
            .expect("a cargo build root");
        let expected_root = build_root.join("tmp");
        assert_eq!(dir.path().parent(), Some(expected_root.as_path()));
        assert!(dir.path().is_dir());
        assert!(!dir.path().starts_with(std::env::temp_dir()));

        let file = NamedTempFile::for_unit_test().expect("unit-test temp file");
        assert_eq!(file.path().parent(), Some(expected_root.as_path()));
    }

    #[test]
    fn an_existing_name_is_a_collision_error_and_a_fresh_one_is_created() {
        let holder = TempDir::for_unit_test().expect("holder");
        let taken = holder.path().join("taken");
        std::fs::create_dir(&taken).expect("occupy the name");

        let error =
            TempDir::create_at(taken.clone()).expect_err("an existing directory is not adopted");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("name collision"), "{error}");
        assert!(
            taken.is_dir(),
            "the refused creation must not remove the existing entry"
        );

        let error = NamedTempFile::create_at(taken).expect_err("an existing entry is not adopted");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("name collision"), "{error}");

        let fresh =
            TempDir::create_at(holder.path().join("fresh")).expect("a fresh name is created");
        assert!(fresh.path().is_dir());
        let fresh_file = NamedTempFile::create_at(holder.path().join("fresh-file"))
            .expect("a fresh name is created");
        assert!(fresh_file.path().is_file());
    }
}
