// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unpacking restores each entry's recorded `files:modified` instant to the
//! nanosecond, per entry: directories, regular files, hardlinks, and targets
//! that are read-only on disk.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_gts::files::{
    FileEntry, FileEntryKind, UnpackOptions, pack_entries_v2, unpack_with_options,
};
use purrdf_gts::reader::read;
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn filesystem_pack_unpack_and_diff_keep_native_paths_and_original_io() {
    use purrdf_gts::files::{diff, pack, unpack};
    let root = purrdf_testkit::temp_dir!("gts-files-native-").expect("temp dir");
    let missing = root.path().join("missing");
    let error = pack(&[&missing]).unwrap_err();
    let events = Vec::with_capacity(4);
    let window = CurrentThreadWindow::open();
    let fields = Capture::read_with_buffer(&error, events);
    assert_eq!(window.close().allocations, 0);
    assert_eq!(fields.code, "gts-files-io");
    assert!(
        matches!(fields.field("path"), ErrorValue::Bytes(value) if value == missing.as_os_str().as_encoded_bytes())
    );
    let cause = std::error::Error::source(&error)
        .unwrap()
        .downcast_ref::<purrdf_hash::error::IoFailure>()
        .unwrap();
    assert_eq!(cause.kind(), std::io::ErrorKind::NotFound);
    assert!(error.to_string().starts_with(&format!("{missing:?}: ")));

    let source = root.path().join("leaf.bin");
    fs::write(&source, b"native files").unwrap();
    let bytes = pack(&[&source]).unwrap();
    let graph = read(&bytes, true, None);
    let dest = root.path().join("extracted");
    unpack(&graph, &dest, false).unwrap();
    assert_eq!(fs::read(dest.join("leaf.bin")).unwrap(), b"native files");
    assert_eq!(diff(&graph, &dest).unwrap(), Vec::<String>::new());
    fs::write(dest.join("leaf.bin"), b"changed").unwrap();
    assert_eq!(diff(&graph, &dest).unwrap(), ["modified: leaf.bin"]);
}

#[test]
fn filesystem_timestamps_keep_original_native_parser_and_epoch_failures() {
    let root = purrdf_testkit::temp_dir!("gts-files-errors-").expect("temp dir");
    let source = root.path().join("before-epoch");
    fs::write(&source, b"data").unwrap();
    // Windows requires a writable handle when changing file timestamps.
    fs::OpenOptions::new()
        .write(true)
        .open(&source)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH - Duration::from_secs(1)))
        .unwrap();
    let error = purrdf_gts::files::pack(&[&source]).unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "gts-files-time-before-epoch");
    assert!(
        matches!(fields.field("path"), ErrorValue::Bytes(value) if value == source.as_os_str().as_encoded_bytes())
    );
    let original = fields.child("cause");
    assert_eq!(original.code, "std-system-time");
    assert!(matches!(original.field("seconds"), ErrorValue::Unsigned(1)));
    assert!(matches!(
        original.field("nanoseconds"),
        ErrorValue::Unsigned(0)
    ));

    let graph = read(
        &pack_entries_v2(&[directory("bad-date", "not-a-date")]).unwrap(),
        true,
        None,
    );
    let error = unpack_with_options(
        &graph,
        &root.path().join("invalid"),
        &UnpackOptions::default(),
    )
    .unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "gts-files-parse-modified");
    assert!(matches!(
        fields.field("actual"),
        ErrorValue::Text("not-a-date")
    ));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .source()
            .unwrap()
            .is::<purrdf_xsd::rfc3339::ParseError>()
    );
}

#[test]
fn extraction_refusals_keep_original_link_targets_and_wide_owner_ids() {
    let root = purrdf_testkit::temp_dir!("gts-extraction-errors-").expect("temp dir");
    let entry = FileEntry {
        path: "sub/link".into(),
        kind: FileEntryKind::Symlink,
        link_target: Some("../../outside".into()),
        ..FileEntry::default()
    };
    let graph = read(&pack_entries_v2(&[entry]).unwrap(), true, None);
    let error = unpack_with_options(
        &graph,
        &root.path().join("links"),
        &UnpackOptions {
            allow_symlinks: true,
            ..UnpackOptions::default()
        },
    )
    .unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "gts-files-symlink-escape");
    assert!(matches!(fields.field("path"), ErrorValue::Text("sub/link")));
    assert!(matches!(
        fields.field("target"),
        ErrorValue::Text("../../outside")
    ));

    let entry = FileEntry {
        path: "owner".into(),
        kind: FileEntryKind::Directory,
        uid: Some(u64::MAX),
        gid: Some(0),
        ..FileEntry::default()
    };
    let graph = read(&pack_entries_v2(&[entry]).unwrap(), true, None);
    let error = unpack_with_options(
        &graph,
        &root.path().join("owners"),
        &UnpackOptions {
            same_owner: true,
            ..UnpackOptions::default()
        },
    )
    .unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "gts-files-unsupported-owner");
    assert!(
        matches!(fields.field("uid"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
    );
    assert!(matches!(fields.field("gid"), ErrorValue::Unsigned(0)));

    for (kind, target) in [
        (FileEntryKind::Symlink, Some("sibling")),
        (FileEntryKind::Fifo, None),
    ] {
        let entry = FileEntry {
            path: "node".into(),
            kind,
            link_target: target.map(str::to_owned),
            ..FileEntry::default()
        };
        let graph = read(&pack_entries_v2(&[entry]).unwrap(), true, None);
        let error = unpack_with_options(
            &graph,
            &root.path().join(kind.as_str()),
            &UnpackOptions {
                allow_symlinks: true,
                allow_special: true,
                ..UnpackOptions::default()
            },
        )
        .unwrap_err();
        let fields = Capture::read(&error).child("cause");
        assert_eq!(fields.code, "gts-files-unsupported-extraction");
        assert!(matches!(fields.field("kind"), ErrorValue::Code(value) if value == kind.as_str()));
        assert!(!root.path().join(kind.as_str()).join("node").exists());
    }
}

#[cfg(unix)]
#[test]
fn filesystem_refusals_keep_non_utf8_paths_and_real_redirecting_ancestors() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    let root = purrdf_testkit::temp_dir!("gts-native-paths-").expect("temp dir");
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    let name = std::ffi::OsString::from_vec(vec![b'n', 255]);
    fs::write(source.join(&name), b"payload").unwrap();
    let error = purrdf_gts::files::pack(&[&source]).unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "gts-files-non-utf8-path");
    assert!(
        matches!(fields.field("path"), ErrorValue::Bytes(value) if value == name.as_encoded_bytes())
    );

    let dest = root.path().join("dest");
    let outside = root.path().join("outside");
    fs::create_dir(&dest).unwrap();
    fs::create_dir(&outside).unwrap();
    symlink(&outside, dest.join("redirect")).unwrap();
    let graph = read(
        &pack_entries_v2(&[FileEntry {
            path: "redirect/leaf".into(),
            data: Some(vec![1]),
            ..FileEntry::default()
        }])
        .unwrap(),
        true,
        None,
    );
    let error = unpack_with_options(&graph, &dest, &UnpackOptions::default()).unwrap_err();
    let fields = Capture::read(&error).child("cause");
    assert_eq!(fields.code, "gts-files-destination-escape");
    assert!(matches!(
        fields.field("path"),
        ErrorValue::Text("redirect/leaf")
    ));
    assert!(
        matches!(fields.field("ancestor"), ErrorValue::Bytes(value) if value == outside.canonicalize().unwrap().as_os_str().as_encoded_bytes())
    );
    assert!(!outside.join("leaf").exists());
}

// Windows file times have 100 ns resolution, so use exactly representable
// fractions while still checking preservation beyond whole seconds.
/// `1971-03-14T15:09:26.535897900Z`.
const DIRECTORY_STAMP: &str = "1971-03-14T15:09:26.535897900Z";
const DIRECTORY_UNIX: (i64, u32) = (37_811_366, 535_897_900);
/// `1969-07-20T20:17:40.123456700Z` — before the epoch, with a fraction.
const LEAF_STAMP: &str = "1969-07-20T20:17:40.123456700Z";
const LEAF_UNIX: (i64, u32) = (-14_182_940, 123_456_700);
/// `1983-11-02T08:00:00.000000100Z`.
const SIBLING_STAMP: &str = "1983-11-02T08:00:00.000000100Z";
const SIBLING_UNIX: (i64, u32) = (436_608_000, 100);
/// `1995-06-30T23:59:59.999999900Z`.
const LINK_STAMP: &str = "1995-06-30T23:59:59.999999900Z";
const LINK_UNIX: (i64, u32) = (804_556_799, 999_999_900);
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
