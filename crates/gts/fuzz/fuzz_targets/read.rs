// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_main]

use libfuzzer_sys::fuzz_target;

// The reader must never panic on arbitrary input: a damaged, truncated, or
// hostile log folds to whatever is recoverable and records diagnostics, never
// crashes. Exercise both single- and multi-segment modes.
fuzz_target!(|data: &[u8]| {
    let _ = purrdf_gts::reader::read(data, false, None);
    let _ = purrdf_gts::reader::read(data, true, None);
});
