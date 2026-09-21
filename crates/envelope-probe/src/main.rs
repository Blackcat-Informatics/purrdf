// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The envelope probe binary: wraps the library workload set with a counting
//! allocator, wall-clock recording, and a machine-readable JSON report.
//!
//! Run under the harness that pins the profile's memory ceiling (a cgroup
//! `MemoryMax` scope on Linux, a pinned `WebAssembly.Memory` maximum for the
//! wasm profile); this binary only measures and reports. Wall time is
//! recorded evidence, never a pass criterion.
//!
//! Usage: `envelope-probe [PROFILE] [--groups N]` — profile defaults to
//! `smoke`; `--groups` overrides the resident scale for exploratory runs.

use std::process::ExitCode;
use std::time::Instant;

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_envelope_probe::{Metric, PROFILES, Profile, WORKLOADS, profile, run};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Resident-set high-water mark in KiB from the OS, `0` where unsupported.
/// Cross-checks the allocator's view; the delta between the two is itself a
/// signal (mmap and allocator slack bypass the counter).
fn rss_kb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if let Some(value) = line
                    .strip_prefix("VmHWM:")
                    .and_then(|rest| rest.split_whitespace().next())
                {
                    return value.parse().unwrap_or(0);
                }
            }
        }
        0
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

fn json_escape(text: &str) -> String {
    use std::fmt::Write as _;
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                let _ = write!(escaped, "\\u{:04x}", control as u32);
            }
            ordinary => escaped.push(ordinary),
        }
    }
    escaped
}

struct WorkloadRow {
    name: &'static str,
    status: Result<Vec<Metric>, String>,
    duration_ms: u128,
    peak_bytes: i64,
    retained_delta_bytes: i64,
}

fn print_report(profile: &Profile, rows: &[WorkloadRow]) {
    println!("{{");
    println!("  \"profile\": \"{}\",", profile.name);
    println!("  \"groups\": {},", profile.groups);
    println!("  \"vm_hwm_kb\": {},", rss_kb());
    println!("  \"workloads\": [");
    for (index, row) in rows.iter().enumerate() {
        let comma = if index + 1 == rows.len() { "" } else { "," };
        print!(
            "    {{\"name\": \"{}\", \"duration_ms\": {}, \"alloc_peak_bytes\": {}, \"retained_delta_bytes\": {}, ",
            row.name, row.duration_ms, row.peak_bytes, row.retained_delta_bytes
        );
        match &row.status {
            Ok(metrics) => {
                print!("\"status\": \"ok\", \"metrics\": {{");
                for (position, (key, value)) in metrics.iter().enumerate() {
                    let inner = if position + 1 == metrics.len() {
                        ""
                    } else {
                        ", "
                    };
                    print!("\"{key}\": {value}{inner}");
                }
                println!("}}}}{comma}");
            }
            Err(message) => {
                println!(
                    "\"status\": \"failed\", \"error\": \"{}\"}}{comma}",
                    json_escape(message)
                );
            }
        }
    }
    println!("  ]");
    println!("}}");
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let mut selected = "smoke".to_owned();
    let mut groups_override: Option<usize> = None;
    while let Some(argument) = arguments.next() {
        if argument == "--groups" {
            let Some(value) = arguments.next().and_then(|v| v.parse().ok()) else {
                eprintln!("envelope-probe: --groups requires a positive integer");
                return ExitCode::from(2);
            };
            groups_override = Some(value);
        } else {
            selected = argument;
        }
    }
    let Some(base) = profile(&selected) else {
        let names: Vec<&str> = PROFILES.iter().map(|p| p.name).collect();
        eprintln!("envelope-probe: unknown profile {selected:?}; profiles: {names:?}");
        return ExitCode::from(2);
    };
    let mut active = *base;
    if let Some(groups) = groups_override {
        active.groups = groups;
    }

    let mut rows = Vec::with_capacity(WORKLOADS.len());
    let mut failed = false;
    for workload in WORKLOADS {
        // The whole-process window: a workload that fans out over worker threads
        // must be charged for what those threads allocate, or the envelope it
        // reports is not the one the memory ceiling has to hold.
        let window = WholeProcessWindow::open();
        let started = Instant::now();
        let status = run(workload, &active, &window);
        let duration_ms = started.elapsed().as_millis();
        let measured = window.close();
        let peak_bytes = measured.peak_working_bytes;
        let retained_delta_bytes = measured.retained_bytes;
        failed |= status.is_err();
        rows.push(WorkloadRow {
            name: workload,
            status,
            duration_ms,
            peak_bytes,
            retained_delta_bytes,
        });
    }
    print_report(&active, &rows);
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::json_escape;

    #[test]
    fn json_escape_covers_control_characters() {
        assert_eq!(
            json_escape("a\"b\\c\nd\te\r\u{1}"),
            "a\\\"b\\\\c\\nd\\te\\r\\u0001"
        );
    }
}
