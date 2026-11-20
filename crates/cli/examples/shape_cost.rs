// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Cost observer for the `shape` verb on a synthetic dataset.
//!
//! `shape_cost gen <TRIPLES> <PATH>` writes a deterministic N-Quads corpus of that many
//! statements: subjects with several predicates, plain, typed and language-tagged literals
//! of varying length, and groups of like blank siblings. Any other argv is the ordinary CLI
//! command line; `PURRDF_SHAPE_COST_RECEIPT` names the file that receives the allocation
//! counts of the whole run, taken through the workspace's shared instrument. Instructions
//! are read from outside with `perf stat -e instructions:u`. This tool measures no time.

use std::fmt::Write as _;

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// The corpus: `triples` statements, deterministic in `triples` alone.
fn corpus(triples: u64) -> String {
    let mut out = String::new();
    let mut state = 1_u64;
    let mut next = move || purrdf_hash::mix::splitmix64_next(&mut state);
    let mut written = 0_u64;
    let mut group = 0_u64;
    while written < triples {
        let subject = next() % (triples / 4 + 1);
        let draw = next();
        let statement = match draw % 8 {
            0..=2 => format!(
                "<http://example.org/s{subject}> <http://example.org/p{}> \"{}\" .\n",
                draw >> 8 & 7,
                "x".repeat((draw >> 16 & 63) as usize)
            ),
            3 | 4 => format!(
                "<http://example.org/s{subject}> <http://example.org/n> \"{}\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
                draw >> 8 & 0xFFFF
            ),
            5 => format!(
                "<http://example.org/s{subject}> <http://example.org/l> \"w{}\"@en .\n",
                draw >> 8 & 0xFFF
            ),
            6 => format!(
                "<http://example.org/s{subject}> <http://example.org/o> <http://example.org/s{}> .\n",
                draw >> 8 & 0xFFFF
            ),
            _ => {
                group += 1;
                let mut text = format!(
                    "<http://example.org/s{subject}> <http://example.org/has> _:g{group} .\n"
                );
                for kid in 0..4 {
                    let _ = writeln!(
                        text,
                        "_:g{group} <http://example.org/kid> _:g{group}k{kid} ."
                    );
                    let _ = writeln!(text, "_:g{group}k{kid} <http://example.org/v> \"same\" .");
                }
                written += 8;
                text
            }
        };
        out.push_str(&statement);
        written += 1;
    }
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("gen") {
        let triples: u64 = args
            .next()
            .and_then(|text| text.parse().ok())
            .expect("gen takes a statement count");
        let path = args.next().expect("gen takes an output path");
        std::fs::write(path, corpus(triples)).expect("write the corpus");
        return;
    }
    let receipt = std::env::var_os("PURRDF_SHAPE_COST_RECEIPT")
        .expect("PURRDF_SHAPE_COST_RECEIPT must name the allocation receipt");
    let window = WholeProcessWindow::open();
    purrdf_cli::run();
    let measured = window.close();
    std::fs::write(
        receipt,
        format!(
            "allocation_calls\t{}\nrequested_bytes\t{}\nretained_bytes\t{}\npeak_working_bytes\t{}\n",
            measured.allocations,
            measured.requested_bytes,
            measured.retained_bytes,
            measured.peak_working_bytes,
        ),
    )
    .expect("write allocation receipt after successful CLI dispatch");
}
