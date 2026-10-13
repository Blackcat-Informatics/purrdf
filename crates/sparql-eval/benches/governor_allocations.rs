// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-process allocation counts; instruction samples use a different executable.

#[cfg(target_os = "linux")]
#[path = "../tests/support/mod.rs"]
mod support;

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn main() {
    #[cfg(target_os = "linux")]
    support::governor_counts::entry(support::governor_counts::CountMode::Allocations);
    #[cfg(not(target_os = "linux"))]
    panic!("native query count collection requires Linux");
}
