// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native archive semantics and exact source/admission boundaries.

use super::{Error, Inventory, Limit, Limits, Problem, read};
use crate::files::{FileEntry, FileEntryKind};
use crate::tar::{HeaderMetadata, ToTarOptions, ZERO_BLOCK, pax_body, write_header, write_octal};
use purrdf_lex::allocation::{Admission, Resident, StorageError};
use purrdf_testkit::error_capture::Capture;
use std::{
    collections::BTreeMap,
    io::{self, Cursor, Read, Seek, SeekFrom},
};

fn limits() -> Limits {
    Limits {
        source_bytes: 1 << 20,
        entries: 1000,
        metadata_bytes: 1 << 16,
        storage_bytes: 1 << 20,
        work: 1 << 24,
    }
}
fn append(tar: &mut Vec<u8>, path: &str, kind: u8, size: u64, bytes: &[u8]) -> usize {
    let start = tar.len();
    let entry = FileEntry {
        path: path.to_owned(),
        ..FileEntry::default()
    };
    let mut metadata = HeaderMetadata::from_entry(&entry, size, &ToTarOptions::default()).unwrap();
    metadata.typeflag = kind;
    metadata.uid = 1;
    write_header(tar, &metadata, bytes).unwrap();
    start
}
fn pax(tar: &mut Vec<u8>, kind: u8, pairs: &[(&str, &str)]) {
    let fields: BTreeMap<_, _> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();
    let body = pax_body(&fields);
    append(tar, "metadata", kind, body.len() as u64, &body);
}
fn finish(tar: &mut Vec<u8>) {
    tar.extend_from_slice(&ZERO_BLOCK);
    tar.extend_from_slice(&ZERO_BLOCK);
}
fn inventory(bytes: &[u8]) -> Inventory<'_> {
    read(bytes, limits(), &mut Resident).expect("valid native archive")
}
fn name(inventory: &Inventory<'_>, index: usize) -> String {
    let path = inventory.entries().nth(index).unwrap().path().unwrap();
    let (prefix, value) = path.utf8_parts().unwrap();
    prefix.map_or_else(|| value.to_owned(), |prefix| format!("{prefix}/{value}"))
}
fn checksum(tar: &mut [u8], start: usize) {
    let header = &mut tar[start..start + 512];
    header[148..156].fill(b' ');
    let sum = header.iter().map(|byte| u64::from(*byte)).sum();
    write_octal(&mut header[148..156], sum).unwrap();
}

#[derive(Default)]
struct Reservation {
    live: usize,
    refuse_refund: bool,
}
impl Admission for Reservation {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        // Native growth may retire old scratch while construction is live.
        // Refuse only the final whole-construction refund in this fixture.
        if self.refuse_refund && bytes == 0 && self.live > 0 {
            return Err(StorageError::AdmissionFailed);
        }
        self.live = bytes;
        Ok(())
    }
}

#[test]
fn inventory_preserves_borrowed_payloads_ustar_fragments_raw_names_and_utf8_causes() {
    let mut tar = Vec::new();
    let prefix = "long-directory/".repeat(8);
    let full = format!("{prefix}leaf");
    let first = append(&mut tar, &full, b'0', 3, b"abc");
    append(&mut tar, "./raw/", b'5', 0, &[]);
    let invalid = append(&mut tar, "invalid", b'0', 0, &[]);
    tar[invalid] = 0xff;
    checksum(&mut tar, invalid);
    finish(&mut tar);
    let view = inventory(&tar);
    let entry = view.entries().next().unwrap();
    let path = entry.path().unwrap();
    assert_eq!(name(&view, 0), full);
    assert_eq!(path.prefix().unwrap().span().start(), first as u64 + 345);
    assert_eq!(path.value().span().start(), first as u64);
    let payload = entry.payload().unwrap();
    assert_eq!(payload, b"abc");
    assert_eq!(payload.as_ptr(), tar[first + 512..first + 515].as_ptr());
    assert_eq!(name(&view, 1), "./raw/");
    let field = view.entries().nth(2).unwrap().path().unwrap().value();
    assert_eq!(field.bytes()[0], 0xff);
    let error = field.utf8().unwrap_err();
    let captured = Capture::read(&error);
    assert_eq!(captured.code, "gts-tar-inventory-utf8");
    assert!(matches!(error, Error::Utf8 { span, .. } if span.start() == invalid as u64));
    assert_eq!(captured.child("cause").code, "std-utf8");
}

#[test]
fn effective_pax_sizes_global_local_deletions_and_gnu_consumption_share_one_framing_law() {
    let mut tar = Vec::new();
    pax(
        &mut tar,
        b'g',
        &[("path", "global"), ("size", "5"), ("uid", "7")],
    );
    append(&mut tar, "gnu-name", b'L', 9, b"gnu-name\0");
    append(&mut tar, "gnu-link", b'K', 9, b"gnu-link\0");
    pax(
        &mut tar,
        b'x',
        &[
            ("path", "local"),
            ("linkpath", "pax-link"),
            ("size", "3"),
            ("uid", "9"),
        ],
    );
    append(&mut tar, "header-one", b'0', 1, b"abc");
    append(&mut tar, "header-two", b'0', 1, b"abcde");
    pax(&mut tar, b'x', &[("path", ""), ("size", ""), ("uid", "")]);
    append(&mut tar, "header-three", b'0', 2, b"zz");
    append(&mut tar, "header-four", b'0', 1, b"hello");
    pax(
        &mut tar,
        b'g',
        &[("path", "replaced"), ("size", "2"), ("uid", "11")],
    );
    append(&mut tar, "header-five", b'0', 1, b"xy");
    pax(
        &mut tar,
        b'g',
        &[("path", ""), ("size", ""), ("uid", ""), ("unseen", "")],
    );
    append(&mut tar, "header-six", b'0', 1, b"z");
    pax(&mut tar, b'g', &[("path", "reinserted"), ("uid", "13")]);
    append(&mut tar, "header-seven", b'0', 1, b"w");
    finish(&mut tar);
    let view = inventory(&tar);
    assert_eq!(
        (0..7).map(|index| name(&view, index)).collect::<Vec<_>>(),
        [
            "local",
            "global",
            "header-three",
            "global",
            "replaced",
            "header-six",
            "reinserted"
        ]
    );
    let entries: Vec<_> = view.entries().collect();
    assert_eq!(entries[0].payload().unwrap(), b"abc");
    assert_eq!(entries[1].payload().unwrap(), b"abcde");
    assert_eq!(entries[2].payload().unwrap(), b"zz");
    assert_eq!(entries[3].payload().unwrap(), b"hello");
    assert_eq!(entries[4].payload().unwrap(), b"xy");
    assert_eq!(entries[5].payload().unwrap(), b"z");
    assert_eq!(entries[6].payload().unwrap(), b"w");
    assert_eq!(
        entries[0].link_target().unwrap().unwrap().utf8().unwrap(),
        "pax-link"
    );
    assert!(
        entries[1].link_target().unwrap().is_none(),
        "PAX consumption must also consume the pending GNU link"
    );
    assert_eq!(entries[0].uid(), 9);
    assert_eq!(entries[1].uid(), 7);
    assert_eq!(entries[2].uid(), 1);
    assert_eq!(entries[3].uid(), 7);
    assert_eq!(entries[4].uid(), 11);
    assert_eq!(entries[5].uid(), 1);
    assert_eq!(entries[6].uid(), 13);
    assert!(entries[5].properties().next().is_none());
    assert_eq!(view.records().len(), 15);
    assert!(
        entries[0]
            .properties()
            .map(Result::unwrap)
            .any(|field| field.key.utf8().unwrap() == "size" && field.value.bytes() == b"3")
    );
}

#[test]
fn gnu_name_and_link_evidence_remain_exact_without_a_pax_override() {
    let mut tar = Vec::new();
    let path = "./gnu-name/";
    let link = "../target";
    append(
        &mut tar,
        "metadata",
        b'L',
        path.len() as u64 + 1,
        format!("{path}\0").as_bytes(),
    );
    append(
        &mut tar,
        "metadata",
        b'K',
        link.len() as u64 + 1,
        format!("{link}\0").as_bytes(),
    );
    append(&mut tar, "placeholder", b'2', 0, &[]);
    finish(&mut tar);
    let view = inventory(&tar);
    assert_eq!(name(&view, 0), path);
    let entry = view.entries().next().unwrap();
    assert_eq!(entry.kind(), FileEntryKind::Symlink);
    assert_eq!(entry.link_target().unwrap().unwrap().utf8().unwrap(), link);
    assert_eq!(entry.path().unwrap().value().span().start(), 512);
}

#[test]
fn malformed_selected_metadata_unknown_records_and_incomplete_archives_never_return_partial_inventory()
 {
    for (key, value) in [
        ("size", "bad"),
        ("size", "+3"),
        ("uid", "-1"),
        ("mtime", "12.bad"),
        ("GNU.sparse.size", "3"),
    ] {
        let mut tar = Vec::new();
        pax(&mut tar, b'x', &[(key, value)]);
        append(&mut tar, "leaf", b'0', 3, b"abc");
        finish(&mut tar);
        assert!(
            read(&tar, limits(), &mut Resident).is_err(),
            "{key}={value}"
        );
    }
    let mut valid = Vec::new();
    pax(
        &mut valid,
        b'x',
        &[("size", "3"), ("mtime", "-12.50"), ("comment", "kept")],
    );
    append(&mut valid, "leaf", b'0', 1, b"abc");
    finish(&mut valid);
    let view = inventory(&valid);
    assert!(
        view.entries()
            .next()
            .unwrap()
            .properties()
            .map(Result::unwrap)
            .any(|field| field.key.bytes() == b"mtime" && field.value.bytes() == b"-12.50")
    );
    let mut unknown = Vec::new();
    append(&mut unknown, "unknown", b'Z', 0, &[]);
    finish(&mut unknown);
    assert!(matches!(
        read(&unknown, limits(), &mut Resident),
        Err(Error::Kind { actual: b'Z', .. })
    ));
    let mut pending = Vec::new();
    append(&mut pending, "metadata", b'L', 4, b"name");
    finish(&mut pending);
    assert!(matches!(
        read(&pending, limits(), &mut Resident),
        Err(Error::Invalid {
            problem: Problem::PendingMetadata,
            ..
        })
    ));
    let mut plain = Vec::new();
    append(&mut plain, "leaf", b'0', 3, b"abc");
    finish(&mut plain);
    for cut in [0, 100, 512, 514, 515, 1023, 1024, 1536] {
        assert!(
            matches!(
                read(&plain[..cut], limits(), &mut Resident),
                Err(Error::Truncated { .. })
            ),
            "cut {cut}"
        );
    }
    inventory(&plain);
}

#[test]
fn exact_source_extension_storage_and_work_admission_have_adjacent_refusals() {
    let mut tar = Vec::new();
    pax(&mut tar, b'x', &[("path", "actual"), ("comment", "value")]);
    append(&mut tar, "placeholder", b'0', 3, b"abc");
    finish(&mut tar);
    let usage = inventory(&tar).usage();
    for kind in [
        Limit::Source,
        Limit::Entries,
        Limit::Metadata,
        Limit::Storage,
        Limit::Work,
    ] {
        let mut exact = limits();
        match kind {
            Limit::Source => exact.source_bytes = usage.source_bytes,
            Limit::Entries => exact.entries = usage.entries,
            Limit::Metadata => exact.metadata_bytes = usage.metadata_bytes,
            Limit::Storage => exact.storage_bytes = usage.storage_bytes,
            Limit::Work => exact.work = usage.work,
        }
        read(&tar, exact, &mut Resident).expect("exact original grant");
        match kind {
            Limit::Source => exact.source_bytes -= 1,
            Limit::Entries => exact.entries -= 1,
            Limit::Metadata => exact.metadata_bytes -= 1,
            Limit::Storage => exact.storage_bytes -= 1,
            Limit::Work => exact.work -= 1,
        }
        let mut reservation = Reservation::default();
        assert!(
            matches!(read(&tar, exact, &mut reservation), Err(Error::Limit { kind: refused, .. }) if refused == kind),
            "{kind:?}"
        );
        assert_eq!(reservation.live, 0, "failed {kind:?} grant is refunded");
    }
    struct Refuse;
    impl Admission for Refuse {
        fn resize(&mut self, _bytes: usize) -> Result<(), StorageError> {
            Err(StorageError::AdmissionFailed)
        }
    }
    assert!(matches!(
        read(&tar, limits(), &mut Refuse),
        Err(Error::Storage {
            cause: StorageError::AdmissionFailed,
            ..
        })
    ));
}

#[test]
fn late_inventory_failure_refunds_original_admission_and_preserves_first_error() {
    let mut tar = Vec::new();
    pax(&mut tar, b'x', &[("path", "leaf"), ("comment", "retained")]);
    append(&mut tar, "placeholder", b'0', 3, b"abc");
    finish(&mut tar);
    let mut malformed = tar.clone();
    *malformed.last_mut().unwrap() = 1;
    for refuse_refund in [false, true] {
        let mut reservation = Reservation {
            refuse_refund,
            ..Reservation::default()
        };
        let error =
            read(&malformed, limits(), &mut reservation).expect_err("the malformed tail refuses");
        assert!(
            matches!(
                error,
                Error::Invalid {
                    problem: Problem::Trailing,
                    ..
                }
            ),
            "refuse_refund={refuse_refund}: {error:?}"
        );
        if refuse_refund {
            assert!(
                reservation.live > 0,
                "the original caller refused the refund"
            );
        } else {
            assert_eq!(reservation.live, 0);
            let accepted = read(&tar, limits(), &mut reservation)
                .expect("the same caller can admit the valid neighbor after refund");
            assert_eq!(
                accepted.entries().next().unwrap().payload().unwrap(),
                b"abc"
            );
            assert!(reservation.live > 0, "successful output retains its grant");
        }
    }
}

#[test]
fn many_pax_keys_preserve_effective_metadata_under_subquadratic_work() {
    fn archive(count: usize) -> Vec<u8> {
        let fields = (0..count)
            .map(|index| (format!("ordinary-{index:04}"), "retained".to_owned()))
            .collect();
        let body = pax_body(&fields);
        let mut tar = Vec::new();
        append(&mut tar, "metadata", b'x', body.len() as u64, &body);
        append(&mut tar, "leaf", b'0', 1, b"x");
        finish(&mut tar);
        tar
    }
    let smaller = archive(128);
    let larger = archive(256);
    let small = inventory(&smaller);
    let large = inventory(&larger);
    assert!(
        large.usage().work < small.usage().work * 3,
        "doubling ordinary PAX keys must avoid quadratic scan work: {} -> {}",
        small.usage().work,
        large.usage().work
    );
    let properties = large.entries().next().unwrap().properties();
    assert_eq!(properties.len(), 256);
    for (index, property) in properties.enumerate() {
        let property = property.unwrap();
        assert_eq!(property.key.utf8().unwrap(), format!("ordinary-{index:04}"));
        assert_eq!(property.value.bytes(), b"retained");
    }
    let options = crate::from_tar::FromTarOptions::default();
    let buffered = crate::from_tar::from_tar_bytes(&larger, &options).unwrap();
    let seekable = crate::from_tar::from_seekable_tar(&mut Cursor::new(&larger), &options).unwrap();
    assert_eq!(buffered, seekable);
    let graph = crate::reader::read(&buffered, false, None);
    assert_eq!(graph.diagnostics, Vec::<crate::model::Diagnostic>::new());
    let entries = crate::files::read_entries(&graph).unwrap();
    assert_eq!(entries["leaf"].pax_records.len(), 256);
}

#[test]
fn buffered_and_seekable_imports_preserve_canonical_outputs_and_explicit_legacy_projection() {
    assert_legacy_framing();
    for (stamp, projected) in [
        ("-12.50", "1970-01-01T00:00:00Z"),
        ("12.50", "1970-01-01T00:00:12Z"),
        (
            "340282366920938463463374607431768211456.50",
            "1970-01-01T00:00:00Z",
        ),
    ] {
        let mut tar = Vec::new();
        pax(
            &mut tar,
            b'x',
            &[
                ("path", "./leaf"),
                ("size", "3"),
                ("mtime", stamp),
                ("comment", "kept"),
            ],
        );
        append(&mut tar, "ignored", b'0', 1, b"abc");
        finish(&mut tar);
        let exact = inventory(&tar);
        assert!(
            exact
                .entries()
                .next()
                .unwrap()
                .properties()
                .map(Result::unwrap)
                .any(|field| field.key.bytes() == b"mtime"
                    && field.value.bytes() == stamp.as_bytes())
        );
        let options = crate::from_tar::FromTarOptions::default();
        let buffered = crate::from_tar::from_tar_bytes(&tar, &options).unwrap();
        let seekable =
            crate::from_tar::from_seekable_tar(&mut Cursor::new(&tar), &options).unwrap();
        assert_eq!(buffered, seekable);
        let graph = crate::reader::read(&buffered, false, None);
        assert_eq!(graph.diagnostics, Vec::<crate::model::Diagnostic>::new());
        let entries = crate::files::read_entries(&graph).unwrap();
        let leaf = &entries["leaf"];
        assert_eq!(leaf.size, Some(3));
        assert_eq!(leaf.modified.as_deref(), Some(projected));
        let canonical = crate::tar::to_tar_vec(&graph, &ToTarOptions::default()).unwrap();
        let repeated = crate::from_tar::from_tar_bytes(&canonical, &options).unwrap();
        assert_eq!(repeated, buffered);
    }
}

/// Legacy tail tolerance never weakens the strict inventory's complete framing.
fn assert_legacy_framing() {
    let mut complete = Vec::new();
    append(&mut complete, "leaf", b'0', 3, b"abc");
    let member = complete.clone();
    finish(&mut complete);
    inventory(&complete);
    let options = crate::from_tar::FromTarOptions::default();
    let expected = crate::from_tar::from_tar_bytes(&complete, &options).unwrap();
    let mut tails = vec![
        member.clone(),
        complete[..member.len() + 512].to_vec(),
        member[..515].to_vec(),
    ];
    let mut nonzero_padding = complete.clone();
    nonzero_padding[515] = 1;
    tails.push(nonzero_padding);
    let mut partial_header = member.clone();
    partial_header.extend_from_slice(b"partial header");
    tails.push(partial_header);
    let mut trailing = complete.clone();
    trailing[member.len() + 512] = 1;
    tails.push(trailing);
    for kind in *b"xLK" {
        let mut pending = member.clone();
        if kind == b'x' {
            pax(&mut pending, kind, &[("path", "unused")]);
        } else {
            append(&mut pending, "metadata", kind, 7, b"unused\0");
        }
        tails.push(pending.clone());
        finish(&mut pending);
        tails.push(pending);
    }
    for tar in tails {
        assert!(read(&tar, limits(), &mut Resident).is_err());
        assert_eq!(
            crate::from_tar::from_tar_bytes(&tar, &options).unwrap(),
            expected
        );
        let mut reader = LegacyReader {
            cursor: Cursor::new(&tar),
            fail_at: None,
        };
        assert_eq!(
            crate::from_tar::from_seekable_tar(&mut reader, &options).unwrap(),
            expected
        );
    }
    for tar in [&[][..], &complete[..100], &complete[..514], &ZERO_BLOCK] {
        assert!(crate::from_tar::from_tar_bytes(tar, &options).is_err());
        assert!(crate::from_tar::from_seekable_tar(&mut Cursor::new(tar), &options).is_err());
    }
    let mut reader = LegacyReader {
        cursor: Cursor::new(member.as_slice()),
        fail_at: Some(member.len() as u64),
    };
    let error = crate::from_tar::from_seekable_tar(&mut reader, &options).unwrap_err();
    let crate::tar::TarError::Io {
        operation, cause, ..
    } = error
    else {
        panic!("{error}")
    };
    assert_eq!(operation, "read tar header");
    assert_eq!(cause.raw_os_error(), Some(13));
}

/// A legacy reader needs start/current seeks and distinguishes header EOF from I/O failure.
struct LegacyReader<'a> {
    cursor: Cursor<&'a [u8]>,
    fail_at: Option<u64>,
}
impl Read for LegacyReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if self
            .fail_at
            .is_some_and(|offset| self.cursor.position() >= offset)
        {
            return Err(io::Error::from_raw_os_error(13));
        }
        self.cursor.read(bytes)
    }
}
impl Seek for LegacyReader<'_> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if matches!(position, SeekFrom::End(_)) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "end-relative seek unavailable",
            ));
        }
        self.cursor.seek(position)
    }
}
