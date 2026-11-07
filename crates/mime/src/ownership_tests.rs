// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Repeated access must use the analyzed owner rather than parse its source again.

use std::cell::Cell;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};

use crate::{Limits, Profile, SourceDocument, Vocabulary, analyze, project};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

thread_local! {
    static ANALYSIS_WORK: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

pub(crate) fn record_analysis(source_bytes: usize) {
    ANALYSIS_WORK.with(|work| {
        let (calls, bytes) = work.get();
        work.set((calls + 1, bytes + source_bytes));
    });
}

fn take_work() -> (usize, usize) {
    ANALYSIS_WORK.with(|work| work.replace((0, 0)))
}

#[test]
fn repeated_part_access_and_projection_reuse_analyzed_source() {
    const ATTACHMENTS: usize = 32;
    let profile = Profile::new(
        "original",
        Vocabulary::under("https://example.org/mime#").unwrap(),
        Limits::unbounded(),
    )
    .unwrap();
    let mut bytes = b"Content-Type: multipart/mixed; boundary=x\r\n\r\n".to_vec();
    for _ in 0..ATTACHMENTS {
        bytes.extend_from_slice(
            b"--x\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\n=41=00=FF\r\n",
        );
    }
    bytes.extend_from_slice(b"--x--\r\n");

    let _ = take_work();
    let window = CurrentThreadWindow::open();
    let document = analyze(
        SourceDocument {
            id: "urn:example:message",
            bytes: &bytes,
        },
        &profile,
    )
    .unwrap();
    let construction = window.close();
    assert_eq!(take_work(), (1, bytes.len()));
    assert_eq!(document.parts().len(), ATTACHMENTS + 1);

    let window = CurrentThreadWindow::open();
    for part in 1..=ATTACHMENTS {
        assert_eq!(document.decoded_attachment(part).unwrap().bytes, b"A\0\xff");
        let cover = document.part_cover(part).unwrap();
        assert_eq!(
            cover.reconstruct().unwrap(),
            &bytes[document.parts()[part].span.clone()]
        );
    }
    let extraction = window.close();
    assert_eq!(
        take_work(),
        (0, 0),
        "part access reanalyzed the whole message"
    );

    let window = CurrentThreadWindow::open();
    let graph = project(&document, &profile).unwrap();
    let projection = window.close();
    assert!(graph.quads().next().is_some());
    assert_eq!(
        take_work(),
        (0, 0),
        "projection reanalyzed the whole message"
    );
    eprintln!(
        "MIME source bytes={}, attachments={ATTACHMENTS}, analysis calls=1; \
         extraction/projection source bytes reanalyzed=0; \
         construction={construction:?}; extraction={extraction:?}; projection={projection:?}",
        bytes.len()
    );
}

#[test]
fn transfer_allocation_tracks_outputs_rather_than_escape_density() {
    let profile = Profile::new(
        "original",
        Vocabulary::under("https://example.org/mime#").unwrap(),
        Limits::unbounded(),
    )
    .unwrap();
    for count in [32, 8192] {
        for (name, encoding, unit, decoded) in [
            (
                "sparse",
                "quoted-printable",
                b"AAAAAAAAAAAAAAA=41".as_slice(),
                b"AAAAAAAAAAAAAAAA".as_slice(),
            ),
            (
                "dense",
                "quoted-printable",
                b"=41".as_slice(),
                b"A".as_slice(),
            ),
            (
                "whitespace",
                "base64",
                b"Y \tW\r\nJj!?".as_slice(),
                b"abc".as_slice(),
            ),
            (
                "binary",
                "binary",
                b"\0\xff\n\r".as_slice(),
                b"\0\xff\n\r".as_slice(),
            ),
        ] {
            let mut source = format!("Content-Transfer-Encoding: {encoding}\r\n\r\n").into_bytes();
            source.extend(unit.repeat(count));
            let expected = decoded.repeat(count);
            let window = CurrentThreadWindow::open();
            let document = analyze(
                SourceDocument {
                    id: "urn:example:transfer",
                    bytes: &source,
                },
                &profile,
            )
            .unwrap();
            let analysis = window.close();
            assert!(document.problems().is_empty());
            let part = &document.parts()[0];
            let window = CurrentThreadWindow::open();
            crate::transfer::visit(&source[part.body.clone()], &part.transfer_encoding, |_| {})
                .unwrap();
            let validation = window.close();
            assert_eq!(
                validation,
                Default::default(),
                "validation retained decoded output"
            );
            let window = CurrentThreadWindow::open();
            let attachment = document.decoded_attachment(0).unwrap();
            let extraction = window.close();
            assert_eq!(attachment.bytes, expected);
            assert!(
                extraction.allocations <= 2,
                "escape density added allocations"
            );
            assert!(extraction.requested_bytes <= 2 * u64::try_from(expected.len()).unwrap());
            let window = CurrentThreadWindow::open();
            let digest = document.decoded_digest(0).unwrap();
            let digest_work = window.close();
            assert_eq!(digest, attachment.digest);
            assert_eq!(
                digest_work,
                Default::default(),
                "digest retained decoded output"
            );
            eprintln!(
                "MIME transfer={name}, input={}, decoded={}, analysis={analysis:?}, validation={validation:?}, extraction={extraction:?}, digest={digest_work:?}",
                unit.len() * count,
                expected.len()
            );
        }
    }
}

#[test]
fn transfer_failures_discard_partial_payloads_and_digests_at_original_offsets() {
    use crate::{MimeError, TransferEncoding as Encoding};
    let profile = Profile::new(
        "original",
        Vocabulary::under("https://example.org/mime#").unwrap(),
        Limits::unbounded(),
    )
    .unwrap();
    for (encoding, body, at, reason) in [
        (
            Encoding::QuotedPrintable,
            b"ok=41=Z0".as_slice(),
            5,
            "invalid quoted-printable hexadecimal escape",
        ),
        (
            Encoding::QuotedPrintable,
            b"ok=\xff0",
            2,
            "non-ASCII quoted-printable escape",
        ),
        (
            Encoding::QuotedPrintable,
            b"x=",
            1,
            "truncated quoted-printable escape",
        ),
        (
            Encoding::QuotedPrintable,
            b"x\n",
            1,
            "illegal unencoded quoted-printable byte or bare line ending",
        ),
        (
            Encoding::Base64,
            b"YWJjAR==",
            0,
            "malformed base64 groups or padding",
        ),
        (
            Encoding::Base64,
            b"YWJjZg==AAAA",
            0,
            "malformed base64 groups or padding",
        ),
    ] {
        let mut source =
            format!("Content-Transfer-Encoding: {}\r\n\r\n", encoding.name()).into_bytes();
        source.extend_from_slice(body);
        let document = analyze(
            SourceDocument {
                id: "urn:example:transfer",
                bytes: &source,
            },
            &profile,
        )
        .unwrap();
        for failure in [
            document.decoded_attachment(0).map(|_| ()),
            document.decoded_digest(0).map(|_| ()),
        ] {
            match failure.unwrap_err() {
                MimeError::Transfer {
                    at: actual,
                    reason: actual_reason,
                    ..
                } => assert_eq!(
                    (actual, actual_reason),
                    (document.parts()[0].body.start + at, reason)
                ),
                other => panic!("unexpected transfer failure: {other}"),
            }
        }
    }
    for (body, decoded) in [
        (b"a=62=\r\nc \t\r\nx \t".as_slice(), b"abc\r\nx".as_slice()),
        (b"=3d=5F", b"=_"),
    ] {
        assert_eq!(
            crate::transfer::decode(body, &Encoding::QuotedPrintable).unwrap(),
            decoded
        );
    }
}
