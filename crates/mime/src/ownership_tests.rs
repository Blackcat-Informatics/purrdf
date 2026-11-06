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
