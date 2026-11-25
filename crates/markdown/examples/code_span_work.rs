// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report deterministic admission work and native allocation for code spans.

use std::borrow::Cow;

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_markdown::{CodeSpanLimits, SyntaxError, replace_code_spans};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn distinct(widths: usize, escaped: bool) -> String {
    let mut source = String::new();
    for width in 1..=widths {
        if escaped && width % 2 == 0 {
            source.push('\\');
        }
        source.extend(std::iter::repeat_n('`', width));
        source.push('x');
    }
    source
}

// The native admission is monotonic in the work ceiling for fixed storage.
// Probe only work refusals. A different refusal invalidates the measurement.
fn charged_work(source: &str, mut limits: CodeSpanLimits) -> u64 {
    let (mut low, mut high) = (0, u64::MAX);
    while low < high {
        limits.work = low + (high - low) / 2;
        match replace_code_spans(source, limits) {
            Ok(_) => high = limits.work,
            Err(SyntaxError::Limit {
                resource: "work", ..
            }) => low = limits.work + 1,
            Err(other) => panic!("unexpected admission failure: {other}"),
        }
    }
    low
}

fn measure(case: &str, source: &str, auxiliary_bytes: usize) {
    let limits = CodeSpanLimits {
        source_bytes: source.len(),
        auxiliary_bytes,
        work: u64::MAX,
    };
    let window = CurrentThreadWindow::open();
    let result = replace_code_spans(source, limits);
    let allocation = window.close();
    let Ok(text) = result else {
        println!(
            "{case} bytes={} allowance={auxiliary_bytes} {result:?}",
            source.len()
        );
        return;
    };
    let work = charged_work(source, limits);
    println!(
        "{case} bytes={} allowance={auxiliary_bytes} work={work} borrowed={} allocations={} requested={} retained={} peak={}",
        source.len(),
        matches!(text, Cow::Borrowed(_)),
        allocation.allocations,
        allocation.requested_bytes,
        allocation.retained_bytes,
        allocation.peak_working_bytes,
    );
}

fn main() {
    const ROOM: usize = 2 * 1024 * 1024;
    measure("plain", &"ordinary text ".repeat(512), ROOM);
    measure(
        "one-unmatched",
        &format!("`{}", "ordinary text ".repeat(512)),
        ROOM,
    );
    measure("paired", &"`x` ".repeat(512), ROOM);
    measure("line-crossing", &"a ``x\r\ny`` b ".repeat(64), ROOM);
    for widths in [16, 32, 64, 128] {
        measure("distinct", &distinct(widths, false), ROOM);
        measure("distinct-escaped", &distinct(widths, true), ROOM);
    }
    let constrained = distinct(64, false);
    for auxiliary_bytes in [0, 128, 1024] {
        measure("constrained", &constrained, auxiliary_bytes);
    }
}
