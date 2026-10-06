// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation budgets for a long stream whose output is much smaller than input.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_deflate::{GzipDecoder, Level, Status, gzip};
use std::fmt::Write as _;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn repeated_blocks_reuse_bounded_working_storage() {
    let mut text = String::with_capacity(4 * 1024 * 1024 + 128);
    let mut subject = 0;
    while text.len() < 4 * 1024 * 1024 {
        writeln!(
            text,
            "<http://example.org/s/{subject}> <http://example.org/p> \"literal {subject:08}\" ."
        )
        .expect("writing to a String");
        subject += 1;
    }
    let input = text.into_bytes();
    drop(gzip::compress(b"warm backend selection", Level::DEFAULT));

    let window = CurrentThreadWindow::open();
    let encoded = gzip::compress(&input, Level::DEFAULT);
    let encode = window.close();
    assert!(encoded.len() < input.len() / 8);
    // Output growth and codec initialization can allocate; block planning
    // must not allocate afresh for every block of this multi-window stream.
    assert!(encode.allocations <= 64, "{encode:?}");
    // The compressed stream and encoder heap buffers fit in one MiB regardless
    // of the four-MiB uncompressed input held outside the window.
    assert!(encode.peak_working_bytes <= 1024 * 1024, "{encode:?}");

    let window = CurrentThreadWindow::open();
    let decoded = gzip::decompress(&encoded).expect("valid gzip");
    let decode = window.close();
    assert_eq!(decoded, input);
    // Output capacity can grow; dynamic decode-table scratch is reused.
    assert!(decode.allocations <= 64, "{decode:?}");
}

#[test]
fn warmed_member_decoding_into_reserved_output_allocates_nothing() {
    let input = b"reused gzip member with a repeated literal and matching history\n".repeat(2048);
    let encoded = gzip::compress(&input, Level::DEFAULT);
    let mut decoder = GzipDecoder::new();
    let mut output = Vec::with_capacity(input.len());
    decoder
        .feed_to_vec(&encoded, &mut output)
        .expect("warm member");
    decoder.finish().expect("complete member");
    assert_eq!(output, input);
    output.clear();

    let window = CurrentThreadWindow::open();
    for part in encoded.chunks(17) {
        let progress = decoder.feed_to_vec(part, &mut output).expect("next member");
        assert_eq!(progress.consumed, part.len());
        assert_eq!(progress.status, Status::NeedsInput);
    }
    decoder.finish().expect("complete second member");
    let measured = window.close();
    assert_eq!(output, input);
    assert_eq!(decoder.members(), 2);
    assert_eq!(measured.allocations, 0, "{measured:?}");
    assert_eq!(measured.requested_bytes, 0, "{measured:?}");
}
