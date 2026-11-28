// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Scratch paths and stand-in executables the bench-lane integration tests share.

// Declared as a module by more than one integration-test binary, and no single binary uses
// every helper; an unused-here helper is used there.
#![allow(dead_code)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// A monotonically increasing counter, so every call to [`unique_tag`] in this process is
/// distinct even across parallel test threads.
static UNIQUE: AtomicU64 = AtomicU64::new(0);

/// A filesystem-unique fragment: this process's id plus a monotonic counter. Parallel test
/// threads within one `cargo test` run (and separate runs, which get different process ids)
/// never collide on the same name.
pub(crate) fn unique_tag() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        UNIQUE.fetch_add(1, Ordering::Relaxed)
    )
}

/// A scratch directory `{prefix}-{label}-{unique tag}` under the system temporary
/// directory, created and returned.
pub(crate) fn scratch(prefix: &str, label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{prefix}-{label}-{}", unique_tag()));
    std::fs::create_dir_all(&dir).expect("create the scratch directory");
    dir
}

/// Writes `contents` to `path` and makes it executable, returning `path`.
pub(crate) fn write_executable(path: PathBuf, contents: &str) -> PathBuf {
    std::fs::write(&path, contents).expect("write the executable script");
    let mut permissions = std::fs::metadata(&path)
        .expect("stat the freshly written script")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("make the script executable");
    path
}
