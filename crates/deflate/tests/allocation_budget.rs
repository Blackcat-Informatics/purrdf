// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation budgets for a long stream whose output is much smaller than input.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_deflate::{GzipDecoder, Inflater, Level, Status, deflate, gzip};
use std::fmt::Write as _;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn bounded_one_shot_decoding_reuses_the_callers_buffer_and_preserves_refusals() {
    let input = b"the original decoded record\n".repeat(256);
    let encoded = deflate::compress(&input, Level::DEFAULT);
    let mut output = Vec::with_capacity(input.len() + 3);
    output.extend_from_slice(b"pre");
    let capacity = output.capacity();
    purrdf_deflate::inflate::decompress_into(&encoded, &mut output, input.len() as u64).unwrap();
    assert_eq!(output.capacity(), capacity);
    assert_eq!(&output[..3], b"pre");
    assert_eq!(&output[3..], input);
    output.clear();
    let limit = u64::try_from(input.len() - 1).unwrap();
    assert_eq!(
        purrdf_deflate::inflate::decompress_into(&encoded, &mut output, limit),
        Err(purrdf_deflate::Error::LimitExceeded { limit })
    );
    output.clear();
    let mut trailing = encoded;
    trailing.push(0);
    assert_eq!(
        purrdf_deflate::inflate::decompress_into(&trailing, &mut output, u64::MAX),
        Err(purrdf_deflate::Error::TrailingData)
    );
}

#[test]
fn raw_decoder_workspace_bound_covers_live_allocations() {
    let input = b"The complete source record retains its original wording and every repeated observation.\n".repeat(2048);
    for level in [Level::NONE, Level::FASTEST, Level::DEFAULT, Level::BEST] {
        let encoded = deflate::compress(&input, level);
        let mut output = vec![0; input.len()];
        let window = CurrentThreadWindow::open();
        let mut decoder = Inflater::new();
        let progress = decoder.feed(&encoded, &mut output).unwrap();
        let measured = window.close();
        assert_eq!(progress.status, Status::StreamEnd);
        assert_eq!(progress.consumed, encoded.len());
        assert_eq!(progress.written, input.len());
        assert_eq!(output, input);
        assert!(
            measured.peak_working_bytes <= i64::try_from(Inflater::max_live_heap_bytes()).unwrap(),
            "{level:?}: {measured:?}"
        );
    }
}

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
