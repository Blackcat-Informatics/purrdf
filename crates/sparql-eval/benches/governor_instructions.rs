// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed-work production query counts with the normal allocator.

#[cfg(target_os = "linux")]
#[path = "../tests/support/mod.rs"]
mod support;

fn main() {
    #[cfg(target_os = "linux")]
    support::governor_counts::entry(support::governor_counts::CountMode::Instructions);
    #[cfg(not(target_os = "linux"))]
    panic!("native instruction counting requires Linux perf");
}
