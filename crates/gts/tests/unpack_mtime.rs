// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unpacking restores each entry's recorded `files:modified` instant to the
//! nanosecond, per entry: directories, regular files, hardlinks, and targets
//! that are read-only on disk.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use purrdf_gts::files::{
    FileEntry, FileEntryKind, UnpackOptions, pack_entries_v2, unpack_with_options,
};
use purrdf_gts::reader::read;

/// `1971-03-14T15:09:26.535897932Z`.
const DIRECTORY_STAMP: &str = "1971-03-14T15:09:26.535897932Z";
const DIRECTORY_UNIX: (i64, u32) = (37_811_366, 535_897_932);
/// `1969-07-20T20:17:40.123456789Z` — before the epoch, with a fraction.
const LEAF_STAMP: &str = "1969-07-20T20:17:40.123456789Z";
const LEAF_UNIX: (i64, u32) = (-14_182_940, 123_456_789);
/// `1983-11-02T08:00:00.000000001Z`.
const SIBLING_STAMP: &str = "1983-11-02T08:00:00.000000001Z";
const SIBLING_UNIX: (i64, u32) = (436_608_000, 1);
/// `1995-06-30T23:59:59.999999999Z`.
const LINK_STAMP: &str = "1995-06-30T23:59:59.999999999Z";
const LINK_UNIX: (i64, u32) = (804_556_799, 999_999_999);
/// `1978-01-01T00:00:00.5Z`.
const READ_ONLY_DIR_STAMP: &str = "1978-01-01T00:00:00.5Z";
const READ_ONLY_DIR_UNIX: (i64, u32) = (252_460_800, 500_000_000);

fn instant((seconds, nanos): (i64, u32)) -> SystemTime {
    let whole = Duration::from_secs(seconds.unsigned_abs());
    let second = if seconds < 0 {
        UNIX_EPOCH - whole
    } else {
        UNIX_EPOCH + whole
    };
    second + Duration::from_nanos(u64::from(nanos))
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path)
        .unwrap_or_else(|e| panic!("stat {path:?}: {e}"))
        .modified()
        .unwrap_or_else(|e| panic!("mtime of {path:?}: {e}"))
}

fn directory(path: &str, stamp: &str) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        kind: FileEntryKind::Directory,
        modified: Some(stamp.to_string()),
        ..FileEntry::default()
    }
}

fn file(path: &str, data: &[u8], stamp: Option<&str>) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        kind: FileEntryKind::File,
        modified: stamp.map(str::to_string),
        data: Some(data.to_vec()),
        ..FileEntry::default()
    }
}

fn unpack_into(dest: &Path, entries: &[FileEntry], options: &UnpackOptions) {
    let archive = pack_entries_v2(entries).expect("pack entries");
    let graph = read(&archive, false, None);
    unpack_with_options(&graph, dest, options).expect("unpack");
}

fn set_read_only(path: &Path, read_only: bool) {
    let mut permissions = fs::metadata(path).expect("stat").permissions();
    permissions.set_readonly(read_only);
    fs::set_permissions(path, permissions).expect("set permissions");
}

#[test]
fn directory_and_files_inside_it_keep_their_own_recorded_instants() {
    let dest = purrdf_testkit::temp_dir!("gts-mtime-tree-").expect("temp dir");
    let entries = [
        directory("tree", DIRECTORY_STAMP),
        file("tree/leaf.txt", b"leaf", Some(LEAF_STAMP)),
        file("tree/sibling.txt", b"sibling", Some(SIBLING_STAMP)),
        // The control: no recorded instant, so its mtime is the write time.
        file("tree/control.txt", b"control", None),
    ];
    unpack_into(dest.path(), &entries, &UnpackOptions::default());

    let root = dest.path().join("tree");
    assert_eq!(modified(&root), instant(DIRECTORY_UNIX));
    assert_eq!(modified(&root.join("leaf.txt")), instant(LEAF_UNIX));
    assert_eq!(modified(&root.join("sibling.txt")), instant(SIBLING_UNIX));

    let control = modified(&root.join("control.txt"));
    for recorded in [DIRECTORY_UNIX, LEAF_UNIX, SIBLING_UNIX] {
        assert_ne!(
            control,
            instant(recorded),
            "the control keeps its own mtime"
        );
    }
    assert!(
        control > instant(LINK_UNIX),
        "the control was stamped at write time, not decades ago"
    );
}

#[test]
fn a_read_only_file_reached_through_a_hardlink_entry_takes_the_recorded_instant() {
    let dest = purrdf_testkit::temp_dir!("gts-mtime-link-").expect("temp dir");
    let original = dest.path().join("original.txt");
    fs::write(&original, b"read-only bytes").expect("seed original");
    set_read_only(&original, true);
    let before = modified(&original);

    let entries = [FileEntry {
        path: "link.txt".to_string(),
        kind: FileEntryKind::Hardlink,
        link_target: Some("original.txt".to_string()),
        modified: Some(LINK_STAMP.to_string()),
        ..FileEntry::default()
    }];
    let options = UnpackOptions {
        allow_symlinks: true,
        ..UnpackOptions::default()
    };
    unpack_into(dest.path(), &entries, &options);

    let link = dest.path().join("link.txt");
    assert!(
        fs::metadata(&link)
            .expect("stat link")
            .permissions()
            .readonly(),
        "the link names the read-only inode"
    );
    assert_ne!(
        before,
        instant(LINK_UNIX),
        "the seed starts at another instant"
    );
    assert_eq!(modified(&link), instant(LINK_UNIX));
    assert_eq!(
        modified(&original),
        instant(LINK_UNIX),
        "one inode, one mtime"
    );

    set_read_only(&original, false);
}

#[test]
fn a_read_only_directory_takes_the_recorded_instant() {
    let dest = purrdf_testkit::temp_dir!("gts-mtime-rodir-").expect("temp dir");
    let locked = dest.path().join("locked");
    fs::create_dir(&locked).expect("seed directory");
    set_read_only(&locked, true);
    // The writable control beside it, recorded at a different instant.
    let entries = [
        directory("locked", READ_ONLY_DIR_STAMP),
        directory("open", DIRECTORY_STAMP),
    ];
    unpack_into(dest.path(), &entries, &UnpackOptions::default());

    assert!(
        fs::metadata(&locked)
            .expect("stat")
            .permissions()
            .readonly(),
        "the directory is still read-only"
    );
    assert_eq!(modified(&locked), instant(READ_ONLY_DIR_UNIX));
    assert_eq!(modified(&dest.path().join("open")), instant(DIRECTORY_UNIX));

    set_read_only(&locked, false);
}
